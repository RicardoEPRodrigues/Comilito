<script lang="ts">
  // The Library drawer along the bottom of the Tapestry view (decision 0017).
  //
  // Open: three columns (Music, Ambience, Effects), each grouped by Library folder, with
  // search, quick filters and tag chips. Closed: a single bar whose search shows results in a
  // pop-up, so a sound can be found and dragged in mid-session without opening anything.
  import {
    AudioLines,
    ChevronDown,
    ChevronRight,
    ChevronUp,
    FilePlus,
    Folder,
    FolderOpen,
    History,
    Layers,
    Link,
    Maximize2,
    Music,
    Plus,
    Search,
    Star,
    Zap,
  } from "@lucide/svelte";
  import type { Component } from "svelte";
  import { api } from "../lib/api";
  import { addSoundFiles } from "../lib/actions";
  import { findSounds, topTags, type Filter } from "../lib/library";
  import { folderOf, remember, run, showView, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category, type Sound } from "../lib/types";
  import SoundRow from "./SoundRow.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  let search = $state("");
  let filter = $state<Filter>("all");
  let searchFocused = $state(false);

  const ICONS: Record<Category, Component> = { music: Music, ambience: AudioLines, effects: Zap };
  const FILTERS: { id: Filter; label: string; icon?: Component }[] = [
    { id: "all", label: "All" },
    { id: "favourites", label: "Favourites", icon: Star },
    { id: "recent", label: "Recent", icon: History },
    { id: "tapestry", label: "In tapestry", icon: Layers },
    { id: "unused", label: "Unused" },
  ];
  const tags = $derived(topTags(project, 4));

  const matches = $derived(findSounds(project, snapshot.missing, { filter, search }));
  const quickResults = $derived(search.trim() ? findSounds(project, snapshot.missing, { filter: "all", search }).slice(0, 8) : []);

  function setOpen(open: boolean) {
    ui.drawerOpen = open;
    remember("drawerOpen", open);
  }

  // Resizing: drag the top edge of the open drawer. The height is kept between a minimum
  // that still shows a few rows and most of the window, and remembered on this computer.
  const MIN_HEIGHT = 160;
  const maxHeight = () => Math.round(window.innerHeight * 0.75);
  let resizing: { startY: number; startHeight: number } | null = null;

  function resizeStart(e: PointerEvent) {
    if (e.button !== 0) return;
    const drawer = (e.currentTarget as HTMLElement).parentElement!;
    resizing = { startY: e.clientY, startHeight: drawer.getBoundingClientRect().height };
    // Pointer capture keeps the moves coming to the handle even when the pointer leaves it.
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function resizeMove(e: PointerEvent) {
    if (!resizing) return;
    // Dragging up (smaller clientY) makes the drawer taller.
    const height = resizing.startHeight + (resizing.startY - e.clientY);
    ui.drawerHeight = Math.round(Math.min(maxHeight(), Math.max(MIN_HEIGHT, height)));
  }
  function resizeEnd() {
    if (!resizing) return;
    resizing = null;
    remember("drawerHeight", ui.drawerHeight);
  }
  function resetHeight() {
    ui.drawerHeight = null;
    remember("drawerHeight", null);
  }

  /** A category's matching sounds, grouped: the category folder first, then each folder. */
  function columns(category: Category): { folder: string; sounds: Sound[] }[] {
    const inCategory = matches.filter((s) => s.category === category);
    const byFolder = new Map<string, Sound[]>();
    for (const folder of ["", ...snapshot.folders[category]]) byFolder.set(folder, []);
    for (const s of inCategory) {
      const f = folderOf(s);
      byFolder.set(f, [...(byFolder.get(f) ?? []), s]);
    }
    const filtering = filter !== "all" || search.trim() !== "";
    return [...byFolder.entries()]
      .map(([folder, sounds]) => ({ folder, sounds }))
      .filter((g) => g.folder === "" ? g.sounds.length > 0 : !filtering || g.sounds.length > 0);
  }

  function folded(category: Category, folder: string) {
    return ui.collapsed.includes(`lib:${category}:${folder}`);
  }

  /** True when a folder above this one is folded, so this one isn't shown at all. */
  function insideFolded(category: Category, folder: string) {
    const parts = folder.split("/");
    for (let i = 1; i < parts.length; i++) {
      if (folded(category, parts.slice(0, i).join("/"))) return true;
    }
    return false;
  }

  function toggleFolder(category: Category, folder: string) {
    const key = `lib:${category}:${folder}`;
    ui.collapsed = ui.collapsed.includes(key) ? ui.collapsed.filter((k) => k !== key) : [...ui.collapsed, key];
    remember("collapsed", ui.collapsed);
  }

  function folderLabel(folder: string) {
    const parts = folder.split("/");
    return { name: parts[parts.length - 1], depth: parts.length - 1 };
  }
</script>

{#snippet searchBox(wide: boolean)}
  <label class="search" class:wide>
    <Search size={14} />
    <input
      data-library-search
      bind:value={search}
      placeholder="Search name or #tag"
      onfocus={() => (searchFocused = true)}
      onblur={() => setTimeout(() => (searchFocused = false), 150)}
      onkeydown={(e) => e.key === "Escape" && ((search = ""), (e.currentTarget as HTMLInputElement).blur())}
    />
    <kbd>/</kbd>
  </label>
{/snippet}

{#snippet tools()}
  <button class="icon" title="From a URL…" onclick={() => (ui.editing = { kind: "addUrl" })}><Link size={16} /></button>
  <button class="icon" title="Open the audio folder in the file manager" onclick={() => run(api.openAudioFolder(null, null))}><FolderOpen size={16} /></button>
  <button class="icon" title="Open the Library workspace" onclick={() => showView("library")}><Maximize2 size={16} /></button>
{/snippet}

<section
  class="drawer"
  class:open={ui.drawerOpen}
  style:height={ui.drawerOpen && ui.drawerHeight ? `min(${ui.drawerHeight}px, 75vh)` : null}
>
  {#if ui.drawerOpen}
    <div
      class="resize"
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize the Library"
      title="Drag to resize · double-click for the default size"
      onpointerdown={resizeStart}
      onpointermove={resizeMove}
      onpointerup={resizeEnd}
      onpointercancel={resizeEnd}
      ondblclick={resetHeight}
    ></div>
  {/if}
  <div class="bar">
    <button class="icon toggle" title={ui.drawerOpen ? "Hide the Library" : "Show the Library"} onclick={() => setOpen(!ui.drawerOpen)}>
      {#if ui.drawerOpen}<ChevronDown size={16} />{:else}<ChevronUp size={16} />{/if}
    </button>
    <h2 class="display">Library</h2>
    <span class="count">{project.sounds.length} sounds</span>
    {@render searchBox(!ui.drawerOpen)}
    {#if ui.drawerOpen}
      <div class="filters">
        {#each FILTERS as f (f.id)}
          <button class="chip" class:on={filter === f.id} onclick={() => (filter = f.id)}>
            {#if f.icon}<f.icon size={12} />{/if}{f.label}
          </button>
        {/each}
        {#if tags.length}<span class="sep"></span>{/if}
        {#each tags as t (t)}
          <button class="chip" class:on={search.trim() === `#${t}`} onclick={() => (search = search.trim() === `#${t}` ? "" : `#${t}`)}>#{t}</button>
        {/each}
      </div>
    {:else}
      <span class="hint">Drag a result onto a scene or the tapestry</span>
    {/if}
    <span class="spacer"></span>
    {@render tools()}

    {#if !ui.drawerOpen && searchFocused && quickResults.length > 0}
      <div class="quick" role="list">
        <span class="caps muted">{quickResults.length} result{quickResults.length === 1 ? "" : "s"} · drag onto a scene or the tapestry</span>
        {#each quickResults as s (s.id)}
          <SoundRow sound={s} detail={`${CATEGORIES.find((c) => c.id === s.category)?.label}${folderOf(s) ? ` › ${folderOf(s)}` : ""}`} />
        {/each}
      </div>
    {/if}
  </div>

  {#if ui.drawerOpen}
    <div class="columns">
      {#each CATEGORIES as c (c.id)}
        {@const Icon = ICONS[c.id]}
        {@const groups = columns(c.id)}
        <div class="column" role="list">
          <div class="col-head">
            <Icon size={14} />
            <span class="caps">{c.label}</span>
            <span class="spacer"></span>
            <span class="count">{matches.filter((s) => s.category === c.id).length}</span>
            <button class="icon" title={`Add ${c.label.toLowerCase()} files…`} onclick={() => addSoundFiles(c.id)}><FilePlus size={14} /></button>
          </div>
          <div class="col-body">
            {#each groups as g (g.folder)}
              {#if g.folder === ""}
                {#each g.sounds as s (s.id)}<SoundRow sound={s} />{/each}
              {:else if !insideFolded(c.id, g.folder)}
                {@const label = folderLabel(g.folder)}
                {@const isFolded = folded(c.id, g.folder)}
                <button class="folder" style:padding-left="{label.depth * 0.9}rem" onclick={() => toggleFolder(c.id, g.folder)}>
                  {#if isFolded}<ChevronRight size={13} />{:else}<ChevronDown size={13} />{/if}
                  {#if isFolded}<Folder size={13} />{:else}<FolderOpen size={13} />{/if}
                  <span class="fname">{label.name}</span>
                  <span class="count">{g.sounds.length}</span>
                </button>
                {#if !isFolded}
                  <div class="nested" style:padding-left="{label.depth * 0.9 + 1.2}rem">
                    {#each g.sounds as s (s.id)}<SoundRow sound={s} />{/each}
                  </div>
                {/if}
              {/if}
            {/each}
            {#if groups.length === 0}
              <p class="empty">
                {#if search || filter !== "all"}Nothing matches.{:else}No {c.label.toLowerCase()} yet.
                  <button class="link" onclick={() => addSoundFiles(c.id)}><Plus size={12} />Add files</button>{/if}
              </p>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .drawer {
    flex: none;
    background: var(--fx-bg);
    border-top: 1px solid var(--rule);
    display: flex;
    flex-direction: column;
    height: 3.6rem;
  }
  .drawer.open {
    height: clamp(14rem, 32vh, 22rem);
    position: relative;
  }
  /* A strip over the top border; taller than it looks, so it's easy to grab. */
  .resize {
    position: absolute;
    top: -4px;
    left: 0;
    right: 0;
    height: 8px;
    cursor: ns-resize;
    z-index: 5;
    touch-action: none;
  }
  .resize:hover {
    background: linear-gradient(transparent 3px, var(--accent) 3px, var(--accent) 5px, transparent 5px);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0 1.2rem;
    height: 3.6rem;
    flex: none;
    position: relative;
  }
  .toggle {
    border: 1px solid var(--rule) !important;
  }
  h2 {
    font-size: 1.6rem;
  }
  .count {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    padding: 0 0.55rem;
    color: var(--muted);
    width: 15rem;
  }
  .search.wide {
    width: 22rem;
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 0.4rem 0;
    outline: none;
  }
  .filters {
    display: flex;
    gap: 0.35rem;
    align-items: center;
    overflow: hidden;
  }
  .sep {
    width: 1px;
    height: 1.1rem;
    background: var(--rule);
  }
  .hint {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .spacer {
    flex: 1;
  }
  .quick {
    position: absolute;
    bottom: calc(100% + 0.4rem);
    left: 12rem;
    width: 26rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: 6px;
    box-shadow: var(--shadow);
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    z-index: 30;
  }
  .quick .caps {
    font-size: 0.58rem;
    padding: 0.1rem 0.3rem 0.35rem;
  }
  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.8rem;
    padding: 0 1.2rem 0.6rem;
  }
  .column {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .col-head {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--accent);
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--rule);
  }
  .col-head .caps {
    font-size: 0.66rem;
  }
  .col-head .icon {
    color: var(--muted);
    padding: 0.15rem;
  }
  .col-body {
    flex: 1;
    overflow: auto;
    padding-top: 0.25rem;
  }
  .folder {
    width: 100%;
    border: none;
    justify-content: flex-start;
    gap: 0.4rem;
    padding: 0.3rem 0.2rem;
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text);
  }
  .folder :global(svg) {
    color: var(--muted);
    flex: none;
  }
  .fname {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    color: var(--muted);
    font-size: 0.8rem;
  }
  .link {
    border: none;
    padding: 0 0.2rem;
    color: var(--accent);
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.8rem;
  }
</style>
