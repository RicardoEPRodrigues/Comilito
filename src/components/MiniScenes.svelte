<script lang="ts">
  // The mini player's drawer of scenes and presets (decision 0049), under its Tapestry.
  // Made to be used fast: a single click adds a scene on top of the tapestry, or loads a preset
  // (the full layout needs a double-click, because a click there selects). Both can also be
  // dragged into the tapestry above. The chevron folds the drawer down to its header.
  // Editing scenes and presets needs the full window, so there are no menus for that here.
  import { ChevronDown, ChevronUp, Layers, Search, X } from "@lucide/svelte";
  import { addSceneOnTop, loadPreset } from "../lib/actions";
  import { project, remember, ui } from "../lib/store.svelte";
  import type { Scene, StackPreset } from "../lib/types";
  import GroupHeading from "./GroupHeading.svelte";

  const p = $derived(project()!);
  const inStack = $derived(new Set(p.stack.filter((e) => e.enabled).map((e) => e.scene)));

  let searching = $state(false);
  let query = $state("");
  const matches = (name: string) => name.toLowerCase().includes(query.trim().toLowerCase());

  /** Items without a group first, then the groups by name; only those matching the search. */
  function grouped<T extends { name: string; group?: string | null }>(items: T[]) {
    const sorted = items.filter((i) => matches(i.name)).sort((a, b) => a.name.localeCompare(b.name));
    const groups = new Map<string, T[]>();
    for (const i of sorted) if (i.group) groups.set(i.group, [...(groups.get(i.group) ?? []), i]);
    return { ungrouped: sorted.filter((i) => !i.group), groups: [...groups.entries()].sort(([a], [b]) => a.localeCompare(b)) };
  }
  const scenes = $derived(grouped<Scene>(p.scenes));
  const presets = $derived(grouped<StackPreset>(p.stackPresets));

  function select(tab: "scenes" | "presets") {
    ui.leftTab = tab;
    remember("leftTab", tab);
  }

  function toggleOpen() {
    ui.miniDrawerOpen = !ui.miniDrawerOpen;
    remember("miniDrawerOpen", ui.miniDrawerOpen);
  }

  function toggleSearch() {
    searching = !searching;
    if (!searching) query = "";
    // Searching needs the list to be visible.
    if (searching && !ui.miniDrawerOpen) toggleOpen();
  }

  function startDrag(e: DragEvent, kind: "scene" | "preset", id: string) {
    ui.drag = { kind, id };
    e.dataTransfer?.setData("text/plain", id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }
</script>

{#snippet sceneRow(scene: Scene)}
  <button
    class="row"
    draggable="true"
    title="Click to add on top of the tapestry · or drag it there"
    ondragstart={(e) => startDrag(e, "scene", scene.id)}
    ondragend={() => (ui.drag = null)}
    onclick={() => addSceneOnTop(scene.id)}
  >
    <span class="dot" class:on={inStack.has(scene.id)}></span>
    <span class="name">{scene.name}</span>
    {#if scene.hotkey}<kbd>{scene.hotkey}</kbd>{/if}
  </button>
{/snippet}

{#snippet presetRow(preset: StackPreset)}
  <button
    class="row"
    class:origin={p.stackOrigin === preset.id}
    draggable="true"
    title="Click to load (replaces the tapestry) · or drag it into the tapestry"
    ondragstart={(e) => startDrag(e, "preset", preset.id)}
    ondragend={() => (ui.drag = null)}
    onclick={() => loadPreset(preset.id)}
  >
    <Layers size={13} />
    <span class="name">{preset.name}</span>
    {#if p.stackOrigin === preset.id}<span class="loaded">loaded</span>{/if}
  </button>
{/snippet}

<section class="drawer" class:open={ui.miniDrawerOpen}>
  <div class="head">
    <button class="icon fold" title={ui.miniDrawerOpen ? "Hide scenes and presets" : "Show scenes and presets"} onclick={toggleOpen}>
      {#if ui.miniDrawerOpen}<ChevronDown size={16} />{:else}<ChevronUp size={16} />{/if}
    </button>
    <div class="tabs" role="tablist">
      <button role="tab" class:active={ui.leftTab === "scenes"} aria-selected={ui.leftTab === "scenes"} onclick={() => select("scenes")}>Scenes</button>
      <button role="tab" class:active={ui.leftTab === "presets"} aria-selected={ui.leftTab === "presets"} onclick={() => select("presets")}>Presets</button>
    </div>
    <span class="spacer"></span>
    <button class="icon" class:on={searching} title={searching ? "Stop searching" : "Search"} onclick={toggleSearch}>
      {#if searching}<X size={14} />{:else}<Search size={14} />{/if}
    </button>
  </div>
  {#if ui.miniDrawerOpen}
    {#if searching}
      <!-- svelte-ignore a11y_autofocus -->
      <input class="search" type="search" placeholder={ui.leftTab === "scenes" ? "Find a scene" : "Find a preset"} bind:value={query} autofocus />
    {/if}
    <div class="list">
      {#if ui.leftTab === "scenes"}
        {#if !p.scenes.length}<p class="empty">No scenes yet. Make them in the full window.</p>{/if}
        {#each scenes.ungrouped as scene (scene.id)}{@render sceneRow(scene)}{/each}
        {#each scenes.groups as [group, items] (group)}
          <GroupHeading id={`scene:${group}`} name={group} count={items.length}>
            {#each items as scene (scene.id)}{@render sceneRow(scene)}{/each}
          </GroupHeading>
        {/each}
      {:else}
        {#if !p.stackPresets.length}<p class="empty">No presets yet. Save the tapestry as one in the full window.</p>{/if}
        {#each presets.ungrouped as preset (preset.id)}{@render presetRow(preset)}{/each}
        {#each presets.groups as [group, items] (group)}
          <GroupHeading id={`preset:${group}`} name={group} count={items.length}>
            {#each items as preset (preset.id)}{@render presetRow(preset)}{/each}
          </GroupHeading>
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .drawer {
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--side-bg);
    color: var(--side-text);
    border-top: 1px solid var(--side-muted);
    min-height: 0;
  }
  /* Open, it takes up to half the room; the tapestry above keeps the rest. */
  .drawer.open {
    flex: 0 1 50%;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem 0;
    border-bottom: 1px solid color-mix(in srgb, var(--side-muted) 60%, transparent);
  }
  .drawer:not(.open) .head {
    border-bottom-color: transparent;
  }
  .head .icon {
    color: var(--side-muted);
    padding: 0.2rem;
    margin-bottom: 0.25rem;
  }
  .head .icon:hover,
  .head .icon.on {
    color: var(--side-text);
  }
  .tabs {
    display: flex;
    gap: 1rem;
  }
  .tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 0.2rem 0 0.4rem;
    font-size: 0.62rem;
    color: var(--side-muted);
  }
  .tabs button:hover {
    background: none !important;
    color: var(--side-text);
  }
  .tabs button.active {
    color: var(--side-text);
    border-bottom-color: var(--side-text);
  }
  .spacer {
    flex: 1;
  }
  .search {
    margin: 0.5rem 0.6rem 0;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.4rem 0.6rem 0.7rem;
  }
  .empty {
    margin: 0.3rem 0;
    color: var(--side-muted);
    font-size: 0.8rem;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.55rem;
    padding: 0.4rem 0.45rem 0.4rem 0.6rem;
    border: 1px solid color-mix(in srgb, var(--side-muted) 70%, transparent);
    border-radius: var(--radius);
    background: none;
    color: var(--side-text);
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.85rem;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: color-mix(in srgb, var(--side-text) 6%, transparent) !important;
  }
  .row.origin {
    border-color: var(--side-text);
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
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  kbd,
  .loaded {
    color: var(--side-muted);
    font-weight: 400;
  }
  .loaded {
    font-size: 0.7rem;
  }
</style>
