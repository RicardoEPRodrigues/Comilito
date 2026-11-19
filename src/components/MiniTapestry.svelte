<script lang="ts">
  // The mini player's Tapestry (decision 0049): the stack, top first, one line per scene.
  // A smaller TapestryPanel without the sound details, the heading or the presets menu.
  // Drag and drop works as there (decision 0014): entries are reordered by dragging them,
  // and scenes or presets dragged from the drawer below are inserted where they're dropped.
  import { GripVertical, Music, Pause, X } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { insertScene } from "../lib/actions";
  import { run, sceneById, ui } from "../lib/store.svelte";
  import type { Scene, StackEntry } from "../lib/types";
  import Capital from "./Capital.svelte";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  /** The stack is stored bottom to top; it's shown top first. */
  const displayed = $derived([...project.stack].reverse());
  const n = $derived(project.stack.length);
  const topEnabled = $derived(displayed.find((e) => e.enabled)?.id);

  /** Where a drop would insert (stack index, 0 = bottom), or null when nothing is dragged here. */
  let dropIndex = $state<number | null>(null);

  $effect(() => {
    if (!ui.drag) dropIndex = null;
  });

  const accepts = () => ui.drag?.kind === "entry" || ui.drag?.kind === "scene" || ui.drag?.kind === "preset";

  /** Displayed position `p` (0 = top): its upper half inserts above it, the lower half below. */
  function overEntry(e: DragEvent, p: number) {
    if (!accepts()) return;
    e.preventDefault();
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropIndex = e.clientY < rect.top + rect.height / 2 ? n - p : n - p - 1;
  }

  /** Over the empty space below the entries: at the bottom of the stack. */
  function overList(e: DragEvent) {
    if (!accepts()) return;
    e.preventDefault();
    if (dropIndex === null) dropIndex = 0;
  }

  async function drop(e: DragEvent) {
    e.preventDefault();
    const drag = ui.drag;
    const index = dropIndex ?? 0;
    dropIndex = null;
    ui.drag = null;
    if (drag?.kind === "scene") await insertScene(drag.id, index);
    else if (drag?.kind === "preset") await run(api.presetInsert(drag.id, index));
    else if (drag?.kind === "entry") await run(api.stackMove(drag.id, index));
  }

  function startDrag(e: DragEvent, entry: StackEntry) {
    ui.drag = { kind: "entry", id: entry.id };
    e.dataTransfer?.setData("text/plain", entry.id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  function musicState(entry: StackEntry, scene: Scene | undefined): "playing" | "paused" | "covered" | null {
    if (!scene?.music?.tracks.length || !entry.enabled) return null;
    if (snapshot.mix.musicEntry === entry.id) return snapshot.playback.transport === "playing" ? "playing" : "paused";
    return "covered";
  }

  function menuFor(entry: StackEntry): MenuItem[] {
    return [
      { label: entry.enabled ? "Disable" : "Enable", action: () => run(api.stackSetEnabled(entry.id, !entry.enabled)) },
      "separator",
      { label: "Remove from tapestry", icon: X, danger: true, action: () => run(api.stackRemove(entry.id)) },
    ];
  }
</script>

<div class="list" role="list" aria-label="Tapestry" ondragover={overList} ondrop={drop}>
  {#if n === 0}
    <p class="empty" class:target={dropIndex !== null}>Click a scene or preset below to start the tapestry.</p>
  {/if}
  {#each displayed as entry, p (entry.id)}
    {@const scene = sceneById(entry.scene)}
    {@const music = musicState(entry, scene)}
    {@const live = entry.id === topEnabled}
    <!-- Mouse-only extras (drag, middle-click, right-click menu); the checkbox and ✕ inside are
         the keyboard way to do the same. -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="entry"
      class:disabled={!entry.enabled}
      class:live
      class:drop-above={dropIndex === n - p}
      class:drop-below={p === n - 1 && dropIndex === 0}
      role="listitem"
      draggable="true"
      title="Drag to reorder · middle-click to enable or disable"
      ondragstart={(e) => startDrag(e, entry)}
      ondragend={() => (ui.drag = null)}
      ondragover={(e) => overEntry(e, p)}
      oncontextmenu={(e) => openMenu(e, menuFor(entry), scene?.name)}
      onmousedown={(e) => e.button === 1 && e.preventDefault()}
      onauxclick={(e) => {
        // Middle click enables or disables, as in the full Tapestry.
        if (e.button !== 1) return;
        e.preventDefault();
        run(api.stackSetEnabled(entry.id, !entry.enabled));
      }}
    >
      <span class="handle"><GripVertical size={14} /></span>
      <input
        type="checkbox"
        checked={entry.enabled}
        onchange={(e) => run(api.stackSetEnabled(entry.id, e.currentTarget.checked))}
        title="Enable / disable"
      />
      <Capital name={scene?.name ?? "?"} size={28} {live} />
      <span class="name">{scene?.name ?? "?"}</span>
      {#if music === "playing"}
        <span class="status playing"><Music size={10} />Playing</span>
      {:else if music === "paused"}
        <span class="status"><Pause size={10} />Paused</span>
      {:else if music === "covered"}
        <span class="status"><Pause size={10} />Covered</span>
      {/if}
      <button class="icon" title="Remove from tapestry" onclick={() => run(api.stackRemove(entry.id))}><X size={15} /></button>
    </div>
  {/each}
</div>

<style>
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.6rem 0.6rem 0.8rem;
  }
  .empty {
    margin: 0;
    padding: 1rem;
    border: 1px dashed var(--rule);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.85rem;
    text-align: center;
  }
  .empty.target {
    border-color: var(--accent);
  }
  .entry {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.4rem 0.35rem 0.45rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    cursor: grab;
    overflow: hidden;
  }
  /* The live (top enabled) scene gets the accent bar on its left edge, as in the Tapestry. */
  .entry.live::before {
    content: "";
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: var(--accent);
  }
  .entry.disabled {
    opacity: 0.55;
  }
  .entry.drop-above {
    box-shadow: 0 -3px 0 var(--accent);
  }
  .entry.drop-below {
    box-shadow: 0 3px 0 var(--accent);
  }
  .handle {
    display: inline-flex;
    color: var(--muted);
  }
  input {
    margin: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    flex: none;
    padding: 0.15rem 0.45rem;
    border: 1px solid var(--rule);
    border-radius: 999px;
    font-size: 0.55rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .status.playing {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .icon {
    color: var(--muted);
    padding: 0.15rem;
  }
</style>
