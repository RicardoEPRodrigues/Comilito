<script lang="ts">
  // Edits a scene: music playlist, ambience layers, effects pad, ducking and transition.
  // Works on a local draft; Save sends the whole scene to Rust.
  import { api } from "../lib/api";
  import { project, run, soundById } from "../lib/store.svelte";
  import { ROLES, type Category, type Role, type Scene } from "../lib/types";
  import Modal from "./Modal.svelte";
  import OverrideField from "./OverrideField.svelte";

  let { original, onclose }: { original: Scene; onclose: () => void } = $props();

  // Copying `original` once is the point: the draft must not follow later changes.
  // svelte-ignore state_referenced_locally
  let draft = $state<Scene>(structuredClone($state.snapshot(original)));

  const p = $derived(project()!);
  const groups = $derived([...new Set(p.scenes.map((s) => s.group).filter(Boolean))] as string[]);

  function soundsOf(category: Category) {
    return p.sounds.filter((s) => s.category === category).sort((a, b) => a.name.localeCompare(b.name));
  }

  function name(id: string) {
    return soundById(id)?.name ?? "?";
  }

  // ---- Music ----
  function addTrack(id: string) {
    if (!id) return;
    if (!draft.music) draft.music = { tracks: [] };
    draft.music.tracks.push(id);
  }
  function removeTrack(i: number) {
    draft.music!.tracks.splice(i, 1);
    if (draft.music!.tracks.length === 0) draft.music = null;
  }
  function moveTrack(i: number, delta: number) {
    const tracks = draft.music!.tracks;
    const j = i + delta;
    if (j < 0 || j >= tracks.length) return;
    [tracks[i], tracks[j]] = [tracks[j], tracks[i]];
  }

  // ---- Hotkey ----
  function captureHotkey(e: KeyboardEvent) {
    if (e.key === "Tab") return;
    e.preventDefault();
    if (e.key === "Backspace" || e.key === "Delete") {
      draft.hotkey = undefined;
      return;
    }
    // Space and Esc are the transport keys; modifier keys alone aren't useful.
    if ([" ", "Escape", "Shift", "Control", "Alt", "Meta", "Enter"].includes(e.key)) return;
    draft.hotkey = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  }
  const hotkeyTakenBy = $derived(
    draft.hotkey
      ? p.scenes.find(
          (s) => s.id !== draft.id && s.hotkey?.toLowerCase() === draft.hotkey!.toLowerCase(),
        )
      : undefined,
  );

  async function save() {
    if (!draft.name.trim()) draft.name = original.name;
    if (draft.group !== undefined && !draft.group.trim()) draft.group = undefined;
    await run(api.updateScene($state.snapshot(draft)));
    onclose();
  }
</script>

<Modal title="Edit scene" {onclose} wide>
  <div class="grid">
    <label>
      <span>Name</span>
      <input bind:value={draft.name} />
    </label>
    <label>
      <span>Group</span>
      <input bind:value={draft.group} list="scene-groups" placeholder="e.g. Wilderness" />
      <datalist id="scene-groups">
        {#each groups as g (g)}<option value={g}></option>{/each}
      </datalist>
    </label>
    <label>
      <span>Hotkey</span>
      <input
        value={draft.hotkey ?? ""}
        onkeydown={captureHotkey}
        placeholder="press a key"
        readonly
        class="hotkey"
      />
      {#if hotkeyTakenBy}<span class="warn">also used by {hotkeyTakenBy.name}</span>{/if}
    </label>
  </div>

  <section>
    <h2>🎵 Music</h2>
    <p class="muted">
      Only the top-most enabled scene with music plays it. Leave empty to keep the music of the
      scenes below.
    </p>
    {#if draft.music}
      <ol>
        {#each draft.music.tracks as id, i (i)}
          <li>
            <span class="grow">{name(id)}</span>
            <button class="icon" onclick={() => moveTrack(i, -1)} disabled={i === 0}>↑</button>
            <button class="icon" onclick={() => moveTrack(i, 1)} disabled={i === draft.music.tracks.length - 1}>↓</button>
            <button class="icon" onclick={() => removeTrack(i)}>✕</button>
          </li>
        {/each}
      </ol>
      <OverrideField
        label="Shuffle"
        kind="toggle"
        value={draft.music.shuffle}
        inherited={p.defaults.categories.music.shuffle ?? false}
        onchange={(v) => (draft.music!.shuffle = v as boolean | undefined)}
      />
    {/if}
    {#if !draft.music}
      <label class="check">
        <input
          type="checkbox"
          checked={draft.blocksMusic ?? false}
          onchange={(e) => (draft.blocksMusic = e.currentTarget.checked || undefined)}
        />
        Silence the music of the scenes below (for example, a tense silence)
      </label>
    {/if}
    <select onchange={(e) => { addTrack(e.currentTarget.value); e.currentTarget.value = ""; }}>
      <option value="">+ Add music track…</option>
      {#each soundsOf("music") as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
    </select>
  </section>

  <section>
    <h2>🌲 Ambience</h2>
    <p class="muted">
      Layers play together. Scenes above duck layers to their duck level, unless the layer has
      a role that no scene above uses.
    </p>
    {#each draft.ambience as layer, i (i)}
      <div class="layer">
        <span class="layer-name">{name(layer.sound)}</span>
        <OverrideField
          label=""
          kind="percent"
          value={layer.volume}
          inherited={soundById(layer.sound)?.volume ?? p.defaults.soundVolume}
          onchange={(v) => (layer.volume = v as number | undefined)}
        />
        <select
          value={layer.role ?? ""}
          onchange={(e) => (layer.role = (e.currentTarget.value || undefined) as Role | undefined)}
        >
          <option value="">no role</option>
          {#each ROLES as r (r)}<option value={r}>{r}</option>{/each}
        </select>
        <button class="icon" onclick={() => draft.ambience.splice(i, 1)}>✕</button>
      </div>
    {/each}
    <select
      onchange={(e) => {
        const id = e.currentTarget.value;
        if (id) draft.ambience.push({ sound: id });
        e.currentTarget.value = "";
      }}
    >
      <option value="">+ Add ambience layer…</option>
      {#each soundsOf("ambience") as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
    </select>
  </section>

  <section>
    <h2>💥 Effects pad</h2>
    <div class="chips">
      {#each draft.effects as id, i (id)}
        <span class="chip">{name(id)} <button class="icon" onclick={() => draft.effects.splice(i, 1)}>✕</button></span>
      {/each}
    </div>
    <select
      onchange={(e) => {
        const id = e.currentTarget.value;
        if (id && !draft.effects.includes(id)) draft.effects.push(id);
        e.currentTarget.value = "";
      }}
    >
      <option value="">+ Add effect…</option>
      {#each soundsOf("effects") as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
    </select>
  </section>

  <section>
    <h2>Mixing</h2>
    <OverrideField
      label="Duck level"
      kind="percent"
      value={draft.duckLevel}
      inherited={p.defaults.duckLevel}
      onchange={(v) => (draft.duckLevel = v as number | undefined)}
    />
    <p class="muted">The volume this scene leaves the ambience below it at (100% = no ducking).</p>
    <OverrideField
      label="Transition"
      kind="seconds"
      value={draft.transitionMs}
      inherited={p.defaults.transitionMs}
      onchange={(v) => (draft.transitionMs = v as number | undefined)}
    />
  </section>

  {#snippet footer()}
    <button onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save}>Save</button>
  {/snippet}
</Modal>

<style>
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 0.8rem 1.2rem;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .check {
    flex-direction: row;
    align-items: center;
    gap: 0.4rem;
  }
  .hotkey {
    width: 7rem;
    cursor: pointer;
  }
  .warn {
    color: var(--warn);
    font-size: 0.8rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border-top: 1px solid var(--border);
    padding-top: 0.8rem;
  }
  section p {
    margin: 0;
  }
  ol {
    margin: 0;
    padding-left: 1.4rem;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }
  .grow {
    flex: 1;
  }
  .layer {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .layer-name {
    width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .layer :global(.label) {
    display: none;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 0 0 0.6rem;
  }
  select {
    align-self: flex-start;
  }
</style>
