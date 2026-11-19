<script lang="ts">
  // The Scenes tab: scene cards in collapsible groups. Drag a scene into the tapestry, or drop
  // a sound from the Library on a scene to add it there. The ⋯ menu (or right-click) has the rest.
  import { Copy, Ellipsis, FolderInput, FolderMinus, FolderPlus, Pencil, Plus, Replace, TextCursorInput, Trash2, ArrowUpToLine } from "@lucide/svelte";
  import { api } from "../lib/api";
  import {
    addSceneOnTop,
    deleteScene,
    duplicateScene,
    moveSceneToNewGroup,
    newScene,
    renameSceneGroup,
    switchTapestryTo,
    ungroupScenes,
  } from "../lib/actions";
  import { focusOn, project, run, ui } from "../lib/store.svelte";
  import type { Scene } from "../lib/types";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";
  import GroupHeading from "./GroupHeading.svelte";
  import SceneContents from "./SceneContents.svelte";

  const p = $derived(project()!);
  const inStack = $derived(new Set(p.stack.filter((e) => e.enabled).map((e) => e.scene)));
  const sorted = $derived([...p.scenes].sort((a, b) => a.name.localeCompare(b.name)));
  const ungrouped = $derived(sorted.filter((s) => !s.group));
  const groups = $derived.by(() => {
    const map = new Map<string, Scene[]>();
    for (const s of sorted) if (s.group) map.set(s.group, [...(map.get(s.group) ?? []), s]);
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  });

  /** "Music · 3 tracks · 2 layers"-style summary, for the card's tooltip (the card shows icons). */
  function meta(s: Scene): string {
    const parts: string[] = [];
    const tracks = s.music?.tracks.length ?? 0;
    if (tracks) parts.push(`Music · ${tracks} track${tracks === 1 ? "" : "s"}`);
    else if (s.blocksMusic) parts.push("Silences music");
    if (s.ambience.length) parts.push(`${s.ambience.length} layer${s.ambience.length === 1 ? "" : "s"}`);
    if (s.effects.length) parts.push(`${s.effects.length} effect${s.effects.length === 1 ? "" : "s"}`);
    if (s.ducksAll) parts.push("Ducks all ambience");
    return parts.join(" · ") || "Empty";
  }

  function menuFor(scene: Scene): MenuItem[] {
    return [
      { label: "Add on top", icon: ArrowUpToLine, action: () => addSceneOnTop(scene.id) },
      { label: "Switch tapestry to this scene", icon: Replace, action: () => switchTapestryTo(scene.id) },
      "separator",
      { label: "Edit", icon: Pencil, action: () => focusOn({ kind: "scene", id: scene.id }) },
      { label: "Duplicate", icon: Copy, action: () => duplicateScene(scene.id) },
      {
        label: "Move to group",
        icon: FolderInput,
        submenu: [
          ...groups.map(([g]) => ({ label: g, checked: scene.group === g, action: () => run(api.sceneSetGroup(scene.id, g)) })),
          { label: "Ungrouped", checked: !scene.group, action: () => run(api.sceneSetGroup(scene.id, null)) },
          "separator",
          { label: "New group…", icon: FolderPlus, action: () => moveSceneToNewGroup(scene.id) },
        ],
      },
      "separator",
      { label: "Delete…", icon: Trash2, danger: true, action: () => deleteScene(scene.id) },
    ];
  }

  function groupMenu(group: string): MenuItem[] {
    return [
      { label: "New scene in this group", icon: Plus, action: () => newScene(group) },
      { label: "Rename group", icon: TextCursorInput, action: () => renameSceneGroup(group) },
      { label: "Ungroup…", icon: FolderMinus, action: () => ungroupScenes(group) },
    ];
  }

  let soundTarget = $state<string | null>(null);

  /** The scene being dragged, if it would actually change group by being dropped in `group`. */
  function sceneMovingTo(group: string | undefined): Scene | null {
    if (ui.drag?.kind !== "scene") return null;
    const id = ui.drag.id;
    const scene = p.scenes.find((s) => s.id === id);
    return scene && (scene.group ?? undefined) !== group ? scene : null;
  }

  /** `undefined` takes the scene out of its group. */
  function dropInGroup(group: string | undefined) {
    const scene = sceneMovingTo(group);
    ui.drag = null;
    if (scene) run(api.sceneSetGroup(scene.id, group ?? null));
  }

  function startDrag(e: DragEvent, id: string) {
    ui.drag = { kind: "scene", id };
    // Some webviews only start a drag if dataTransfer holds something.
    e.dataTransfer?.setData("text/plain", id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }
</script>

{#snippet card(scene: Scene)}
  <div
    class="card"
    class:target={soundTarget === scene.id}
    class:editing={ui.focus?.kind === "scene" && ui.focus.id === scene.id}
    role="button"
    tabindex="0"
    draggable="true"
    title={`${meta(scene)}\nDrag into the tapestry · double-click to add on top`}
    ondragstart={(e) => startDrag(e, scene.id)}
    ondragend={() => (ui.drag = null)}
    ondragover={(e) => {
      // A scene dragged onto another card joins that card's group (or leaves its group, on an
      // ungrouped card). Group headings take drops too.
      if (sceneMovingTo(scene.group)) {
        e.preventDefault();
        if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
        return;
      }
      if (ui.drag?.kind !== "sound" && ui.drag?.kind !== "video") return;
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      soundTarget = scene.id;
    }}
    ondragleave={() => (soundTarget = null)}
    ondrop={(e) => {
      if (sceneMovingTo(scene.group)) {
        e.preventDefault();
        dropInGroup(scene.group);
        return;
      }
      const drag = ui.drag;
      if (drag?.kind !== "sound" && drag?.kind !== "video") return;
      e.preventDefault();
      soundTarget = null;
      ui.drag = null;
      // A video joins the scene's video playlist; a sound goes where its category goes.
      run(drag.kind === "video" ? api.addVideoToScene(scene.id, drag.id) : api.addSoundToScene(scene.id, drag.id));
    }}
    ondblclick={() => addSceneOnTop(scene.id)}
    oncontextmenu={(e) => openMenu(e, menuFor(scene), scene.name)}
    onkeydown={(e) => e.key === "Enter" && addSceneOnTop(scene.id)}
  >
    <span class="dot" class:on={inStack.has(scene.id)} title={inStack.has(scene.id) ? "In the tapestry" : ""}></span>
    <span class="text">
      <span class="name">{scene.name}</span>
      <span class="meta"><SceneContents {scene} /></span>
    </span>
    {#if scene.hotkey}<kbd>{scene.hotkey}</kbd>{/if}
    <button class="icon" title="Scene options" onclick={(e) => openMenu(e, menuFor(scene), scene.name)}><Ellipsis size={16} /></button>
  </div>
{/snippet}

<div class="list">
  {#if p.scenes.length === 0}
    <p class="empty">No scenes yet. Create one, then drag sounds from the Library into it.</p>
  {/if}
  {#each ungrouped as scene (scene.id)}{@render card(scene)}{/each}
  {#each groups as [group, scenes] (group)}
    <GroupHeading
      id={`scene:${group}`}
      name={group}
      count={scenes.length}
      onmenu={(e) => openMenu(e, groupMenu(group), group)}
      droppable={sceneMovingTo(group) !== null}
      ondrop={() => dropInGroup(group)}
    >
      {#each scenes as scene (scene.id)}{@render card(scene)}{/each}
    </GroupHeading>
  {/each}
</div>
<div class="foot">
  <button onclick={() => newScene()}><Plus size={14} />New scene</button>
</div>

<style>
  .list {
    flex: 1;
    overflow: auto;
    padding: 0.2rem 1.1rem 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .empty {
    color: var(--side-muted);
    font-size: 0.85rem;
    line-height: 1.45;
  }
  .card {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.4rem 0.5rem 0.65rem;
    border: 1px solid color-mix(in srgb, var(--side-muted) 70%, transparent);
    border-radius: var(--radius);
    cursor: grab;
    color: var(--side-text);
  }
  .card:hover {
    background: color-mix(in srgb, var(--side-text) 6%, transparent);
  }
  .card.target {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .card.editing {
    border-color: var(--side-text);
    border-width: 1.5px;
  }
  .dot {
    flex: none;
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    border: 1px solid var(--side-muted);
  }
  .dot.on {
    background: var(--side-text);
    border-color: var(--side-text);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 0.72rem;
    color: var(--side-muted);
  }
  kbd {
    color: var(--side-muted);
  }
  .icon {
    color: var(--side-muted);
  }
  .foot {
    padding: 0.8rem 1.1rem 1rem;
  }
  .foot button {
    width: 100%;
    justify-content: center;
    border-color: var(--side-text);
  }
</style>
