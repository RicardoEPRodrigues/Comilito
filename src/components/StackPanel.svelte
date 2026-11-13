<script lang="ts">
  // The scene stack, top of the stack shown at the top (docs/scenes.md).
  //
  // Drag and drop uses the browser's built-in HTML5 events (decision 0014):
  //   dragstart (on the dragged item) → dragover (on whatever it's over, many times a second)
  //   → drop (on the target) → dragend (on the dragged item, always, even if cancelled).
  // A target must call `preventDefault()` in dragover, or the browser won't allow a drop.
  //
  // The stack is stored bottom to top, so displayed position `p` (0 = top) of `n` entries is
  // stack index `n - 1 - p`. "Insert above displayed item p" means stack index `n - p`.
  import { api } from "../lib/api";
  import { insertScene, savePreset } from "../lib/actions";
  import { percent, run, sceneById, soundById, ui } from "../lib/store.svelte";
  import type { StackEntry } from "../lib/types";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  const displayed = $derived([...project.stack].reverse());
  const n = $derived(project.stack.length);

  /** Where a drop would insert (stack index, 0 = bottom), or null when not dragging over us. */
  let dropIndex = $state<number | null>(null);
  /** True while an entry is dragged outside the stack, so releasing it removes it. */
  let draggedOutside = $state(false);
  let panel: HTMLElement;

  let expanded = $state<Set<string>>(new Set());

  function toggleExpanded(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expanded = next;
  }

  /** The entry a dragged sound is over (sounds are added to a scene, not inserted). */
  let soundTarget = $state<string | null>(null);

  // When any drag ends (here or elsewhere, dropped or cancelled), clear the drop indicators.
  $effect(() => {
    if (!ui.drag) {
      dropIndex = null;
      soundTarget = null;
    }
  });

  function onItemDragOver(e: DragEvent, p: number, entry: StackEntry) {
    if (!ui.drag) return;
    e.preventDefault();
    e.stopPropagation();
    if (ui.drag.kind === "sound") {
      soundTarget = entry.id;
      dropIndex = null;
      return;
    }
    // Upper half of the item: insert above it; lower half: below it.
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const upper = e.clientY < rect.top + rect.height / 2;
    dropIndex = upper ? n - p : n - p - 1;
  }

  function onPanelDragOver(e: DragEvent) {
    // Sounds can only be dropped on an entry, not between them.
    if (!ui.drag || ui.drag.kind === "sound") return;
    e.preventDefault();
    // Over empty space below the entries: insert at the bottom.
    if (dropIndex === null) dropIndex = 0;
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    const drag = ui.drag;
    const index = dropIndex ?? 0;
    const target = soundTarget;
    dropIndex = null;
    soundTarget = null;
    ui.drag = null;
    if (!drag) return;

    if (drag.kind === "sound") {
      // Add the sound to the scene of the entry it was dropped on.
      const entry = project.stack.find((en) => en.id === target);
      if (entry) await run(api.addSoundToScene(entry.scene, drag.id));
      return;
    }

    if (drag.kind === "scene") await insertScene(drag.id, index);
    else if (drag.kind === "preset") await run(api.presetInsert(drag.id, index));
    else await run(api.stackMove(drag.id, index));
  }

  function onEntryDragStart(e: DragEvent, entry: StackEntry) {
    ui.drag = { kind: "entry", id: entry.id };
    draggedOutside = false;
    e.dataTransfer?.setData("text/plain", entry.id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  function onEntryDragEnd(entry: StackEntry) {
    // Released outside the stack: "drag it out of the stack" removes it.
    if (draggedOutside) run(api.stackRemove(entry.id));
    draggedOutside = false;
    dropIndex = null;
    ui.drag = null;
  }

  /** Watches drags anywhere in the window to know when an entry leaves the stack. */
  function onWindowDragOver(e: DragEvent) {
    if (ui.drag?.kind !== "entry" || !panel) return;
    const r = panel.getBoundingClientRect();
    const inside = e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    draggedOutside = !inside;
    if (!inside) dropIndex = null;
  }

  function musicStatus(entry: StackEntry): string {
    const scene = sceneById(entry.scene);
    if (!scene?.music?.tracks.length) return "";
    if (snapshot.mix.musicEntry === entry.id) {
      return snapshot.playback.transport === "playing" ? "🎵▶" : "🎵⏸";
    }
    return "🎵⏸";
  }
</script>

<!-- In Edit mode, a ✕ that takes a sound out of the scene (it stays in the library). -->
{#snippet removeButton(sceneId: string, soundId: string)}
  {#if ui.mode === "edit"}
    <button
      class="icon"
      title="Remove from this scene"
      onclick={() => run(api.removeSoundFromScene(sceneId, soundId))}>✕</button
    >
  {/if}
{/snippet}

<svelte:window ondragover={onWindowDragOver} />

<section
  bind:this={panel}
  class="stack"
  class:dragging={ui.drag !== null}
  ondragover={onPanelDragOver}
  ondragleave={(e) => {
    // Only clear when leaving the panel itself, not when moving between its children.
    if (!panel.contains(e.relatedTarget as Node)) {
      dropIndex = null;
      soundTarget = null;
    }
  }}
  ondrop={onDrop}
  role="list"
  aria-label="Scene stack"
>
  <div class="head">
    <h2>Stack <span class="muted">▲ top</span></h2>
    {#if snapshot.playback.transport === "stopped" && n > 0}
      <span class="hint">Press <kbd>Space</kbd> to start</span>
    {/if}
  </div>

  <div class="entries">
    {#if n === 0}
      <div class="empty" class:target={dropIndex !== null}>
        Drag scenes or presets here, or double-click one in the library.
      </div>
    {/if}

    {#each displayed as entry, p (entry.id)}
      {@const scene = sceneById(entry.scene)}
      {@const layerVolumes = snapshot.mix.layerVolumes[entry.id] ?? []}
      <div
        class="entry"
        class:disabled={!entry.enabled}
        class:drop-above={dropIndex === n - p}
        class:drop-below={p === n - 1 && dropIndex === 0}
        class:drop-into={soundTarget === entry.id}
        role="listitem"
        ondragover={(e) => onItemDragOver(e, p, entry)}
      >
        <!-- Only the header row is draggable: if the whole entry were, dragging a volume
             slider in the expanded details would drag the entry instead. -->
        <div
          class="row"
          role="presentation"
          draggable="true"
          ondragstart={(e) => onEntryDragStart(e, entry)}
          ondragend={() => onEntryDragEnd(entry)}
        >
          <span class="handle" title="Drag to reorder, or out of the stack to remove">⠿</span>
          <input
            type="checkbox"
            checked={entry.enabled}
            onchange={(e) => run(api.stackSetEnabled(entry.id, e.currentTarget.checked))}
            title="Enable / disable"
          />
          <button class="name" onclick={() => toggleExpanded(entry.id)}>
            {scene?.name ?? "?"}
          </button>
          <span class="status">
            {#if !entry.enabled}
              <span class="muted">disabled</span>
            {:else}
              {musicStatus(entry)}
              {#if scene?.blocksMusic && !scene.music?.tracks.length}<span title="Silences the music below">🔇</span>{/if}
              {#if scene?.ambience.length}🌲×{scene.ambience.length}{/if}
              {#if scene?.effects.length}💥×{scene.effects.length}{/if}
            {/if}
          </span>
          <button class="icon" title="Remove from stack" onclick={() => run(api.stackRemove(entry.id))}>✕</button>
        </div>

        {#if expanded.has(entry.id) && scene}
          <div class="details">
            {#if scene.music?.tracks.length}
              <div class="tracks">
                {#each scene.music.tracks as trackId (trackId)}
                  <div
                    class="item"
                    class:playing={snapshot.mix.musicEntry === entry.id && snapshot.playback.musicSound === trackId}
                  >
                    <span class="layer-name">🎵 {soundById(trackId)?.name ?? "?"}</span>
                    {@render removeButton(scene.id, trackId)}
                  </div>
                {/each}
              </div>
            {/if}
            {#each scene.ambience as layer, i (i)}
              {@const sound = soundById(layer.sound)}
              {@const base = layer.volume ?? sound?.volume ?? project.defaults.soundVolume}
              <div class="layer">
                <span class="layer-name" title={layer.role ?? "no role"}>
                  🌲 {sound?.name ?? "?"}
                  {#if layer.role}<span class="muted">· {layer.role}</span>{/if}
                </span>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={Math.round(base * 100)}
                  oninput={(e) => run(api.setLayerVolume(scene.id, i, e.currentTarget.valueAsNumber / 100))}
                  title="Volume in this scene"
                />
                <span class="vol" title="Effective volume after ducking">
                  {entry.enabled ? percent(layerVolumes[i] ?? 0) : "—"}
                </span>
                {@render removeButton(scene.id, layer.sound)}
              </div>
            {/each}
            {#each scene.effects as effectId (effectId)}
              <div class="item">
                <span class="layer-name">💥 {soundById(effectId)?.name ?? "?"}</span>
                {@render removeButton(scene.id, effectId)}
              </div>
            {/each}
            {#if !scene.music?.tracks.length && scene.ambience.length === 0 && scene.effects.length === 0}
              <span class="muted">This scene has no sounds yet.</span>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="foot">
    <span class="muted">▼ bottom</span>
    <button onclick={savePreset} disabled={n === 0}>Save stack as preset</button>
  </div>
</section>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 0.6rem 0.8rem;
    gap: 0.5rem;
  }
  .head,
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .hint {
    color: var(--accent);
  }
  kbd {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 0.3rem;
  }
  .entries {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.2rem 0;
  }
  .empty {
    border: 2px dashed var(--border);
    border-radius: 8px;
    padding: 2rem;
    text-align: center;
    color: var(--muted);
  }
  .empty.target {
    border-color: var(--accent);
  }
  .entry {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    /* Room for the drop indicator line, so it doesn't make the list jump. */
    border-top-width: 3px;
    border-bottom-width: 3px;
    border-top-color: transparent;
    border-bottom-color: transparent;
  }
  .entry.drop-above {
    border-top-color: var(--accent);
  }
  .entry.drop-below {
    border-bottom-color: var(--accent);
  }
  .entry.drop-into {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, var(--panel));
  }
  .entry.disabled {
    opacity: 0.55;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem;
  }
  .handle {
    cursor: grab;
    color: var(--muted);
  }
  .name {
    flex: 1;
    text-align: left;
    background: none;
    border: none;
    font-weight: 600;
    padding: 0.2rem 0;
  }
  .status {
    color: var(--muted);
    font-size: 0.85rem;
    display: flex;
    gap: 0.4rem;
  }
  .details {
    border-top: 1px solid var(--border);
    padding: 0.4rem 0.8rem 0.6rem 2.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .tracks div.playing {
    color: var(--accent);
    font-weight: 600;
  }
  .layer {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .layer-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .layer input {
    width: 8rem;
  }
  .vol {
    width: 3rem;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
</style>
