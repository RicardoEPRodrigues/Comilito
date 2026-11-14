<script lang="ts">
  // Edits a scene in place in the centre panel (decision 0017): music playlist, ambience
  // layers, effects pad and settings. Works on a local draft; Save (in the context bar) sends
  // the whole scene to Rust. Sounds can be dragged in from the Library drawer.
  import { ArrowDown, ArrowUp, ArrowUpToLine, AudioLines, Music, Plus, X, Zap } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { onDestroy } from "svelte";
  import { addSceneOnTop, stopPreviewOnLeave } from "../lib/actions";
  import { project, run, sceneById, soundById, ui } from "../lib/store.svelte";
  import { ROLES, roleLabel, type Category, type Role, type Scene } from "../lib/types";
  import { confirmLeave, fingerprint } from "../lib/unsaved";
  import Capital from "./Capital.svelte";
  import OverrideField from "./OverrideField.svelte";
  import Watermark from "./Watermark.svelte";

  let { id }: { id: string } = $props();
  // Sounds previewed from this editor stop when it closes.
  onDestroy(() => stopPreviewOnLeave());

  // svelte-ignore state_referenced_locally
  const original = sceneById(id)!;
  // Copying the scene once is the point: the draft must not follow later changes.
  let draft = $state<Scene>(structuredClone($state.snapshot(original)));
  let saved = $state(fingerprint($state.snapshot(original)));
  const dirty = $derived(fingerprint($state.snapshot(draft)) !== saved);

  const p = $derived(project()!);
  const groups = $derived([...new Set(p.scenes.map((s) => s.group).filter(Boolean))] as string[]);
  const usedIn = $derived.by(() => {
    const inStack = p.stack.some((e) => e.scene === id);
    const presets = p.stackPresets.filter((pr) => pr.stack.some((e) => e.scene === id)).length;
    const parts = [];
    if (inStack) parts.push("in the tapestry");
    if (presets) parts.push(`in ${presets} preset${presets === 1 ? "" : "s"}`);
    return parts.length ? `Used ${parts.join(" and ")}` : "Not used yet";
  });

  async function save() {
    if (!draft.name.trim()) draft.name = original.name;
    if (draft.group !== undefined && !draft.group.trim()) draft.group = undefined;
    const snapshot = $state.snapshot(draft);
    // The volumes are set from the mixer bar, maybe while this editor was open: keep the
    // current ones rather than the draft's copy from when it opened.
    snapshot.volumes = $state.snapshot(sceneById(id)?.volumes);
    await run(api.updateScene(snapshot));
    saved = fingerprint(snapshot);
  }

  function discard() {
    const current = sceneById(id) ?? original;
    draft = structuredClone($state.snapshot(current));
    saved = fingerprint($state.snapshot(current));
  }

  // Hand Save / Discard to the context bar, and guard against leaving with unsaved changes.
  $effect(() => {
    ui.editorActions = { dirty, save, discard, saveLabel: "Save scene" };
  });
  $effect(() => {
    const guard = () => confirmLeave(dirty, save);
    ui.guard = guard;
    return () => {
      if (ui.guard === guard) ui.guard = null;
    };
  });

  function name(soundId: string) {
    return soundById(soundId)?.name ?? "?";
  }

  // ---- Adding sounds (drag from the Library, or the pickers) ----
  function addSound(soundId: string) {
    const sound = soundById(soundId);
    if (!sound) return;
    if (sound.category === "music") {
      if (!draft.music) draft.music = { tracks: [] };
      if (!draft.music.tracks.includes(soundId)) draft.music.tracks.push(soundId);
    } else if (sound.category === "ambience") {
      if (!draft.ambience.some((l) => l.sound === soundId)) draft.ambience.push({ sound: soundId, role: "foreground" });
    } else if (!draft.effects.includes(soundId)) {
      draft.effects.push(soundId);
    }
  }
  let dropOver = $state(false);

  function soundsOf(category: Category) {
    return p.sounds.filter((s) => s.category === category).sort((a, b) => a.name.localeCompare(b.name));
  }

  // ---- Music ----
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
    e.stopPropagation();
    if (e.key === "Backspace" || e.key === "Delete") {
      draft.hotkey = undefined;
      return;
    }
    // Space, Esc and / are app keys; modifier keys alone aren't useful.
    if ([" ", "Escape", "/", "Shift", "Control", "Alt", "Meta", "Enter"].includes(e.key)) return;
    draft.hotkey = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  }
  const hotkeyTakenBy = $derived(
    draft.hotkey
      ? p.scenes.find((s) => s.id !== draft.id && s.hotkey?.toLowerCase() === draft.hotkey!.toLowerCase())
      : undefined,
  );
</script>

{#snippet picker(category: Category, label: string)}
  <select
    class="picker"
    onchange={(e) => {
      addSound(e.currentTarget.value);
      e.currentTarget.value = "";
    }}
  >
    <option value="">+ {label}</option>
    {#each soundsOf(category) as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
  </select>
{/snippet}

<div
  class="editor"
  class:drop-over={dropOver}
  role="region"
  aria-label="Scene editor"
  ondragover={(e) => {
    if (ui.drag?.kind !== "sound") return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    dropOver = true;
  }}
  ondragleave={(e) => {
    // WebKitGTK gives no relatedTarget: check whether the pointer really left.
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    if (e.clientX <= r.left || e.clientX >= r.right || e.clientY <= r.top || e.clientY >= r.bottom) dropOver = false;
  }}
  ondrop={(e) => {
    if (ui.drag?.kind !== "sound") return;
    e.preventDefault();
    addSound(ui.drag.id);
    ui.drag = null;
    dropOver = false;
  }}
>
  <Watermark />
  <div class="head">
    <Capital name={draft.name} size={68} live />
    <div class="title">
      <input class="name display" bind:value={draft.name} aria-label="Scene name" />
      <span class="caps muted">Scene{draft.group ? ` · ${draft.group}` : ""} · {usedIn}</span>
    </div>
    <button onclick={() => addSceneOnTop(id)} title="Add this scene on top of the tapestry"><ArrowUpToLine size={14} />Add on top</button>
  </div>

  {#if dropOver}<div class="drop-banner"><Plus size={14} />Drop to add the sound to this scene (saved when you press Save)</div>{/if}

  <section>
    <div class="section-head"><span class="caps">Music</span><span class="hint">Only the top-most enabled scene with music plays it</span></div>
    {#each draft.music?.tracks ?? [] as trackId, i (i)}
      <div class="row">
        <Music size={15} />
        <span class="grow">{name(trackId)}</span>
        <button class="icon" onclick={() => moveTrack(i, -1)} disabled={i === 0} title="Move up"><ArrowUp size={14} /></button>
        <button class="icon" onclick={() => moveTrack(i, 1)} disabled={i === (draft.music?.tracks.length ?? 0) - 1} title="Move down"><ArrowDown size={14} /></button>
        <button class="icon" onclick={() => removeTrack(i)} title="Remove"><X size={15} /></button>
      </div>
    {/each}
    <div class="under">
      {@render picker("music", "Add music track")}
      {#if draft.music}
        <OverrideField
          label=""
          kind="toggle"
          value={draft.music.shuffle}
          inherited={p.defaults.categories.music.shuffle ?? false}
          onchange={(v) => (draft.music!.shuffle = v as boolean | undefined)}
        />
        <span class="muted small">Shuffle</span>
      {:else}
        <label class="switch">
          <input type="checkbox" checked={draft.blocksMusic ?? false} onchange={(e) => (draft.blocksMusic = e.currentTarget.checked || undefined)} />
          <span class="small">Silence the music of the scenes below (a tense silence)</span>
        </label>
      {/if}
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Ambience layers</span><span class="hint">A role decides which layers a scene above ducks</span></div>
    {#each draft.ambience as layer, i (i)}
      <div class="row">
        <AudioLines size={15} />
        <span class="layer-name">{name(layer.sound)}</span>
        <select class="role" value={layer.role} onchange={(e) => (layer.role = e.currentTarget.value as Role)}>
          {#each ROLES as r (r)}<option value={r}>{roleLabel(r)}</option>{/each}
        </select>
        <div class="grow">
          <OverrideField
            label=""
            kind="percent"
            value={layer.volume}
            inherited={soundById(layer.sound)?.volume ?? p.defaults.soundVolume}
            onchange={(v) => (layer.volume = v as number | undefined)}
          />
        </div>
        <button class="icon" onclick={() => draft.ambience.splice(i, 1)} title="Remove"><X size={15} /></button>
      </div>
    {/each}
    <div class="under">{@render picker("ambience", "Add ambience layer")}<span class="muted small">or drag sounds here from the Library</span></div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Effects pad</span><span class="hint">Shown on the right while this scene is in the tapestry</span></div>
    <div class="chips">
      {#each draft.effects as effectId, i (effectId)}
        <span class="chip"><Zap size={12} />{name(effectId)}<button class="icon x" onclick={() => draft.effects.splice(i, 1)} title="Remove"><X size={12} /></button></span>
      {/each}
      {@render picker("effects", "Add effect")}
    </div>
  </section>

  <section>
    <div class="section-head"><span class="caps">Settings</span><span class="hint">Grey values follow the project defaults</span></div>
    <div class="settings">
      <OverrideField
        label="Ducks scenes below to"
        kind="percent"
        value={draft.duckLevel}
        inherited={p.defaults.duckLevel}
        onchange={(v) => (draft.duckLevel = v as number | undefined)}
      />
      <OverrideField
        label="Transition"
        kind="seconds"
        value={draft.transitionMs}
        inherited={p.defaults.transitionMs}
        onchange={(v) => (draft.transitionMs = v as number | undefined)}
      />
      <label class="field">
        <span class="caps">Hotkey</span>
        <input value={draft.hotkey ?? ""} onkeydown={captureHotkey} placeholder="press a key" readonly class="hotkey" />
        {#if hotkeyTakenBy}<span class="warn">also used by {hotkeyTakenBy.name}</span>{/if}
      </label>
      <label class="field">
        <span class="caps">Group</span>
        <input bind:value={draft.group} list="scene-groups" placeholder="e.g. Wilderness" />
        <datalist id="scene-groups">
          {#each groups as g (g)}<option value={g}></option>{/each}
        </datalist>
      </label>
    </div>
  </section>
</div>

<style>
  .editor {
    flex: 1;
    overflow: auto;
    padding: 1.5rem 2.4rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.4rem;
    position: relative;
  }
  .editor > :not(:global(.watermark)) {
    position: relative;
    z-index: 1;
  }
  .editor.drop-over {
    outline: 2px dashed var(--accent);
    outline-offset: -8px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 1.1rem;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .name {
    font-size: 2.6rem;
    background: transparent;
    border: 1px solid transparent;
    padding: 0 0.2rem;
    margin-left: -0.25rem;
    color: var(--text);
  }
  .name:hover,
  .name:focus {
    border-color: var(--rule);
  }
  .title .caps {
    font-size: 0.62rem;
  }
  .drop-banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--accent);
    color: var(--accent-text);
    padding: 0.55rem 0.8rem;
    border-radius: var(--radius);
    font-weight: 600;
    font-size: 0.85rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.6rem 0.5rem 0.8rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .layer-name {
    width: 11rem;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .role {
    width: 9rem;
    padding: 0.3rem 0.5rem;
    border-radius: 999px;
    font-size: 0.75rem;
  }
  .under {
    display: flex;
    align-items: center;
    gap: 0.8rem;
  }
  .picker {
    width: auto;
    font-size: 0.82rem;
    background: transparent;
    border-style: dashed;
  }
  .small {
    font-size: 0.8rem;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    align-items: center;
  }
  .chip :global(svg) {
    color: var(--accent);
  }
  .x {
    padding: 0.05rem;
  }
  .settings {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
    gap: 0.8rem;
  }
  .hotkey {
    cursor: pointer;
    font-weight: 600;
  }
  .warn {
    color: var(--warn);
    font-size: 0.75rem;
  }
</style>
