<script lang="ts">
  // Left panel: presets and scenes (drag them into the stack), and the sound library.
  import { api } from "../lib/api";
  import {
    addSceneOnTop,
    deleteScene,
    newScene,
    renamePreset,
  } from "../lib/actions";
  import { run, ui } from "../lib/store.svelte";
  import type { Scene } from "../lib/types";
  import { openMenu } from "./ContextMenu.svelte";
  import SoundLibrary from "./SoundLibrary.svelte";

  let tab = $state<"scenes" | "sounds">("scenes");

  const project = $derived(ui.snapshot!.project!);
  const editMode = $derived(ui.mode === "edit");

  // Scenes grouped by their `group` field; scenes without one come first.
  const groups = $derived.by(() => {
    const map = new Map<string, Scene[]>();
    for (const scene of [...project.scenes].sort((a, b) => a.name.localeCompare(b.name))) {
      const key = scene.group ?? "";
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(scene);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  });

  const inStack = $derived(new Set(project.stack.map((e) => e.scene)));

  function sceneMenu(e: MouseEvent, scene: Scene) {
    openMenu(e, [
      { label: "Add on top", action: () => addSceneOnTop(scene.id) },
      { label: "Switch stack to this scene", action: () => run(api.stackSwitch(scene.id)) },
      ...(editMode
        ? [
            { label: "Edit…", action: () => (ui.editing = { kind: "scene", id: scene.id }) },
            {
              label: "Duplicate",
              action: async () => {
                const id = await run(api.duplicateScene(scene.id));
                if (id) ui.editing = { kind: "scene", id };
              },
            },
            { label: "Delete…", action: () => deleteScene(scene.id), danger: true },
          ]
        : []),
    ]);
  }

  function presetMenu(e: MouseEvent, presetId: string, name: string) {
    openMenu(e, [
      { label: "Replace stack", action: () => run(api.presetLoad(presetId)) },
      { label: "Add on top", action: () => run(api.presetInsert(presetId, project.stack.length)) },
      { label: "Rename…", action: () => renamePreset(presetId, name) },
      { label: "Delete", action: () => run(api.presetDelete(presetId)), danger: true },
    ]);
  }

  function startDrag(e: DragEvent, kind: "scene" | "preset", id: string) {
    ui.drag = { kind, id };
    // Some webviews only start a drag if dataTransfer holds something.
    e.dataTransfer?.setData("text/plain", id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }
</script>

<aside>
  <div class="tabs" role="tablist">
    <button role="tab" class:active={tab === "scenes"} onclick={() => (tab = "scenes")}>Scenes</button>
    <button role="tab" class:active={tab === "sounds"} onclick={() => (tab = "sounds")}>Sounds</button>
  </div>

  {#if tab === "scenes"}
    <div class="scroll">
      {#if project.stackPresets.length > 0}
        <h2>Presets</h2>
        <ul>
          {#each project.stackPresets as preset (preset.id)}
            <li>
              <div
                class="item"
                role="button"
                tabindex="0"
                draggable="true"
                ondragstart={(e) => startDrag(e, "preset", preset.id)}
                ondragend={() => (ui.drag = null)}
                ondblclick={() => run(api.presetLoad(preset.id))}
                oncontextmenu={(e) => presetMenu(e, preset.id, preset.name)}
                title="Drag into the stack, double-click to replace the stack, right-click for more"
              >
                <span class="icon">▤</span>
                <span class="name">{preset.name}</span>
                <span class="muted">{preset.stack.length}</span>
              </div>
            </li>
          {/each}
        </ul>
      {/if}

      <h2>Scenes</h2>
      {#if project.scenes.length === 0}
        <p class="muted hint">
          No scenes yet. Add sounds in the Sounds tab, then switch to Edit mode and create a scene.
        </p>
      {/if}
      {#each groups as [group, scenes] (group)}
        {#if group}
          <h3>{group}</h3>
        {/if}
        <ul>
          {#each scenes as scene (scene.id)}
            <li>
              <div
                class="item"
                class:in-stack={inStack.has(scene.id)}
                role="button"
                tabindex="0"
                draggable="true"
                ondragstart={(e) => startDrag(e, "scene", scene.id)}
                ondragend={() => (ui.drag = null)}
                ondblclick={() => addSceneOnTop(scene.id)}
                oncontextmenu={(e) => sceneMenu(e, scene)}
                title="Drag into the stack, double-click to add on top, right-click for more"
              >
                <span class="name">{scene.name}</span>
                <span class="badges">
                  {#if scene.music?.tracks.length}🎵{:else if scene.blocksMusic}🔇{/if}
                  {#if scene.ambience.length}🌲{scene.ambience.length}{/if}
                  {#if scene.effects.length}💥{scene.effects.length}{/if}
                </span>
                {#if scene.hotkey}<kbd>{scene.hotkey}</kbd>{/if}
                {#if editMode}
                  <button
                    class="icon"
                    title="Edit scene"
                    onclick={() => (ui.editing = { kind: "scene", id: scene.id })}>✎</button
                  >
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/each}
    </div>
    {#if editMode}
      <div class="bottom">
        <button onclick={newScene}>+ New scene</button>
      </div>
    {/if}
  {:else}
    <SoundLibrary />
  {/if}
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--panel);
  }
  .tabs {
    display: flex;
    gap: 0.3rem;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    flex: 1;
  }
  .tabs button.active {
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  h3 {
    font-size: 0.85rem;
    color: var(--muted);
    margin-top: 0.3rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    cursor: grab;
  }
  .item:hover {
    border-color: var(--accent);
  }
  .item.in-stack {
    border-left: 3px solid var(--accent);
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badges {
    font-size: 0.8rem;
    color: var(--muted);
  }
  kbd {
    font-size: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 0.3rem;
    color: var(--muted);
  }
  .hint {
    margin: 0;
  }
  .bottom {
    padding: 0.5rem;
    border-top: 1px solid var(--border);
  }
</style>
