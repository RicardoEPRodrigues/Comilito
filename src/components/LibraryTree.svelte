<script lang="ts">
  // Left column of the Library workspace (decision 0018): smart lists, then each category with
  // its folders (real subfolders of audio/<category>/). Drop sounds on a folder to move them
  // there, or drag a folder onto another (or onto its category) to move it inside. The
  // downloads queue sits at the bottom. Videos (decision 0035) are a fourth shelf with folders
  // of their own, after the three audio categories.
  import { fitText } from "../lib/fit";
  import {
    AudioLines,
    ChevronDown,
    ChevronRight,
    CircleDashed,
    FilePlus,
    Film,
    Folder,
    FolderOpen,
    FolderInput,
    FolderPlus,
    History,
    Library,
    Link,
    Music,
    Pencil,
    Search,
    Star,
    Trash2,
    TriangleAlert,
    Zap,
  } from "@lucide/svelte";
  import type { Component } from "svelte";
  import { api } from "../lib/api";
  import { addSoundFiles, foldersOf, moveFolder, moveSounds, moveTargets, newFolder } from "../lib/actions";
  import { addVideoFiles, folderOfVideo, videoEnabled } from "../lib/video";
  import { fold } from "../lib/motion";
  import { askText, folderOf, remember, run, ui, type SmartList } from "../lib/store.svelte";
  import { CATEGORIES, type Shelf } from "../lib/types";
  import { openMenu, type MenuItem } from "./ContextMenu.svelte";
  import Downloads from "./Downloads.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  const lib = ui.library;

  const ICONS: Record<Shelf, Component> = { music: Music, ambience: AudioLines, effects: Zap, video: Film };
  /** The shelves shown: the audio categories, and Video unless it's turned off. */
  const SHELVES = $derived<{ id: Shelf; label: string }[]>([
    ...CATEGORIES,
    ...(videoEnabled() ? [{ id: "video" as const, label: "Video" }] : []),
  ]);
  const shelfLabel = (shelf: Shelf) => SHELVES.find((s) => s.id === shelf)?.label ?? shelf;
  const SMART: { id: SmartList; label: string; icon: Component }[] = [
    { id: "all", label: "All sounds", icon: Library },
    { id: "favourites", label: "Favourites", icon: Star },
    { id: "recent", label: "Recent", icon: History },
    { id: "unused", label: "Unused", icon: CircleDashed },
    { id: "missing", label: "Missing files", icon: TriangleAlert },
  ];

  const used = $derived.by(() => {
    const ids = new Set<string>();
    for (const s of project.scenes) {
      s.music?.tracks.forEach((t) => ids.add(t));
      s.ambience.forEach((l) => ids.add(l.sound));
      s.effects.forEach((e) => ids.add(e));
    }
    return ids;
  });
  const cutoff = Date.now() - 30 * 24 * 3600 * 1000;
  function smartCount(id: SmartList): number {
    const s = project.sounds;
    switch (id) {
      case "all":
        return s.length;
      case "favourites":
        return s.filter((x) => x.favourite).length;
      case "recent":
        return s.filter((x) => x.lastUsed && Date.parse(x.lastUsed) >= cutoff).length;
      case "unused":
        return s.filter((x) => !used.has(x.id)).length;
      case "missing":
        // Sounds only: missing videos are marked in the Video shelf.
        return project.sounds.filter((x) => snapshot.missing.includes(x.id)).length;
    }
  }

  function countIn(category: Shelf, folder: string | null) {
    if (category === "video") {
      const inFolder = (f: string) => folder === null || f === folder || f.startsWith(`${folder}/`);
      return (project.videos ?? []).filter((v) => inFolder(folderOfVideo(v))).length;
    }
    return project.sounds.filter(
      (s) => s.category === category && (folder === null || folderOf(s) === folder || folderOf(s).startsWith(`${folder}/`)),
    ).length;
  }

  function selectSmart(id: SmartList) {
    lib.smart = id;
    lib.category = null;
    lib.folder = null;
    lib.cropping = false;
  }

  function selectFolder(category: Shelf, folder: string | null) {
    lib.smart = "all";
    lib.category = category;
    lib.folder = folder;
    lib.cropping = false;
  }

  function isOpen(category: Shelf) {
    return !ui.collapsed.includes(`tree:${category}`);
  }
  function toggle(category: Shelf) {
    const key = `tree:${category}`;
    ui.collapsed = ui.collapsed.includes(key) ? ui.collapsed.filter((k) => k !== key) : [...ui.collapsed, key];
    remember("collapsed", ui.collapsed);
  }

  async function renameFolder(category: Shelf, folder: string) {
    const parts = folder.split("/");
    const name = await askText("Rename folder", "Name", parts[parts.length - 1]);
    if (!name) return;
    const to = [...parts.slice(0, -1), name].join("/");
    await run(api.renameFolder(category, folder, to));
    if (lib.category === category && lib.folder === folder) lib.folder = to;
  }

  async function deleteFolder(category: Shelf, folder: string) {
    await run(api.deleteFolder(category, folder));
    if (lib.category === category && lib.folder === folder) lib.folder = null;
  }

  function addFromUrl(category: Shelf, folder: string) {
    ui.editing = { kind: "addUrl", target: { category, folder } };
  }

  /** "Add files…" picks audio or video files, depending on the shelf. */
  function addFilesTo(category: Shelf, folder: string | null) {
    return category === "video" ? addVideoFiles(folder) : addSoundFiles(category, folder);
  }

  function folderMenu(category: Shelf, folder: string): MenuItem[] {
    const label = shelfLabel(category);
    const targets = moveTargets(category, folder);
    return [
      { label: "Add files here…", icon: FilePlus, action: () => addFilesTo(category, folder) },
      { label: "Add from URL here…", icon: Link, action: () => addFromUrl(category, folder) },
      { label: "New folder inside…", icon: FolderPlus, action: () => newFolder(category, folder) },
      { label: "Rename…", icon: Pencil, action: () => renameFolder(category, folder) },
      {
        label: "Move to",
        icon: FolderInput,
        disabled: targets.length === 0,
        submenu: targets.map((t) => ({
          label: t === "" ? `${label} (top level)` : t,
          action: () => moveFolder(category, folder, t),
        })),
      },
      { label: "Show in file manager", icon: FolderOpen, action: () => run(api.openAudioFolder(category, folder)) },
      "separator",
      { label: "Delete (if empty)", icon: Trash2, danger: true, action: () => deleteFolder(category, folder) },
    ];
  }

  function categoryMenu(category: Shelf): MenuItem[] {
    return [
      { label: "Add files…", icon: FilePlus, action: () => addFilesTo(category, null) },
      { label: "Add from URL…", icon: Link, action: () => addFromUrl(category, "") },
      { label: "New folder…", icon: FolderPlus, action: () => newFolder(category) },
      { label: "Show in file manager", icon: FolderOpen, action: () => run(api.openAudioFolder(category, null)) },
    ];
  }

  // Dropping sounds on a folder (or category) moves them there; so does dropping a folder,
  // within its own category (a folder's sounds can't change category by being dragged).
  let dropTarget = $state<string | null>(null);
  function accepts(category: Shelf, folder: string): boolean {
    const d = ui.drag;
    // Sounds go to audio folders, videos to video folders.
    if (d?.kind === "sound") return category !== "video";
    if (d?.kind === "video") return category === "video";
    return d?.kind === "folder" && d.category === category && moveTargets(category, d.folder).includes(folder);
  }
  function dragOver(e: DragEvent, category: Shelf, folder: string) {
    if (!accepts(category, folder)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dropTarget = `${category}:${folder}`;
  }
  function folderDragStart(e: DragEvent, category: Shelf, folder: string) {
    ui.drag = { kind: "folder", category, folder };
    e.dataTransfer?.setData("text/plain", folder);
    // As for sounds: WebKit refuses a drop unless "move" is allowed.
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }
  async function drop(e: DragEvent, category: Shelf, folder: string) {
    const d = ui.drag;
    if (!d || !accepts(category, folder)) return;
    e.preventDefault();
    ui.drag = null;
    dropTarget = null;
    if (d.kind === "folder") {
      await moveFolder(category, d.folder, folder);
      return;
    }
    if (d.kind !== "sound" && d.kind !== "video") return;
    const dragged = d.id;
    // Dragging one of several selected items moves the whole selection.
    const ids = lib.selected.includes(dragged) ? [...lib.selected] : [dragged];
    if (d.kind === "video") await run(api.moveVideos(ids, folder));
    else if (category !== "video") await moveSounds(ids, category, folder);
  }

  function addFiles() {
    addFilesTo(lib.category ?? "ambience", lib.category ? lib.folder : null);
  }
</script>

<aside>
  <div class="head">
    <h2 class="display" use:fitText>Library</h2>
    <span class="count">{project.sounds.length}</span>
  </div>
  <div class="actions">
    <button onclick={addFiles} title="Copy audio files into the project"><FilePlus size={13} />Add files</button>
    <button onclick={() => (ui.editing = { kind: "addUrl" })}><Link size={13} />From URL</button>
    <button class="icon framed" title="Open the audio folder in the file manager" onclick={() => run(api.openAudioFolder(lib.category, lib.category ? lib.folder : null))}>
      <FolderOpen size={15} />
    </button>
  </div>
  <label class="search">
    <Search size={14} />
    <input data-library-search bind:value={lib.search} placeholder="Search name or #tag" />
  </label>

  <div class="scroll">
    <div class="group">
      {#each SMART as s (s.id)}
        {@const selected = lib.category === null && lib.smart === s.id}
        <button class="node" class:selected class:warn={s.id === "missing" && snapshot.missing.length > 0} onclick={() => selectSmart(s.id)}>
          <s.icon size={14} /><span class="label">{s.label}</span><span class="n">{smartCount(s.id)}</span>
        </button>
      {/each}
    </div>
    <hr />
    <div class="group">
      {#each SHELVES as c (c.id)}
        {@const Icon = ICONS[c.id]}
        {@const open = isOpen(c.id)}
        <button
          class="node strong"
          class:selected={lib.category === c.id && lib.folder === null}
          class:drop={dropTarget === `${c.id}:` || ui.fileDropTarget === `${c.id}:`}
          data-drop-category={c.id}
          data-drop-folder=""
          onclick={() => selectFolder(c.id, null)}
          oncontextmenu={(e) => openMenu(e, categoryMenu(c.id), c.label)}
          ondragover={(e) => dragOver(e, c.id, "")}
          ondragleave={() => (dropTarget = null)}
          ondrop={(e) => drop(e, c.id, "")}
        >
          <span class="chev" role="presentation" onclick={(e) => (e.stopPropagation(), toggle(c.id))}>
            {#if open}<ChevronDown size={13} />{:else}<ChevronRight size={13} />{/if}
          </span>
          <Icon size={14} /><span class="label">{c.label}</span><span class="n">{countIn(c.id, null)}</span>
        </button>
        {#if open}
          <div class="folders" transition:fold>
            {#each foldersOf(c.id) as folder (folder)}
              {@const parts = folder.split("/")}
              {@const selected = lib.category === c.id && lib.folder === folder}
              <button
                class="node"
                class:selected
                class:drop={dropTarget === `${c.id}:${folder}` || ui.fileDropTarget === `${c.id}:${folder}`}
                data-drop-category={c.id}
                data-drop-folder={folder}
                style:padding-left="{1.7 + (parts.length - 1) * 0.9}rem"
                onclick={() => selectFolder(c.id, folder)}
                oncontextmenu={(e) => openMenu(e, folderMenu(c.id, folder), parts[parts.length - 1])}
                draggable="true"
                ondragstart={(e) => folderDragStart(e, c.id, folder)}
                ondragend={() => ((ui.drag = null), (dropTarget = null))}
                ondragover={(e) => dragOver(e, c.id, folder)}
                ondragleave={() => (dropTarget = null)}
                ondrop={(e) => drop(e, c.id, folder)}
              >
                {#if selected}<FolderOpen size={14} />{:else}<Folder size={14} />{/if}
                <span class="label">{parts[parts.length - 1]}</span><span class="n">{countIn(c.id, folder)}</span>
              </button>
            {/each}
          </div>
        {/if}
      {/each}
      <button class="node new" onclick={() => newFolder(lib.category ?? "ambience", lib.category ? lib.folder : null)}>
        <FolderPlus size={13} /><span class="label">New folder{lib.category ? "" : " (in Ambience)"}</span>
      </button>
    </div>
  </div>

  <Downloads />
</aside>

<style>
  aside {
    width: 17.5rem;
    flex: none;
    background: var(--side-bg);
    color: var(--side-text);
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    padding: 1.1rem 0.9rem 0.9rem;
    min-height: 0;
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 0 0.2rem;
  }
  h2 {
    font-size: 1.9rem;
  }
  .count {
    font-size: 0.75rem;
    color: var(--side-muted);
  }
  .actions {
    display: flex;
    gap: 0.4rem;
  }
  .actions button {
    flex: 1;
    justify-content: center;
    border-color: var(--side-text);
    padding: 0.4rem 0.3rem;
    font-size: 0.62rem;
  }
  .actions .framed {
    flex: none;
    border: 1px solid var(--side-text);
    padding: 0.35rem;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    border: 1px solid var(--side-muted);
    border-radius: var(--radius);
    padding: 0 0.55rem;
    color: var(--side-muted);
  }
  .search input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 0.4rem 0;
    color: var(--side-text);
    outline: none;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .group,
  .folders {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  hr {
    border: none;
    border-top: 1px solid color-mix(in srgb, var(--side-muted) 45%, transparent);
    margin: 0.5rem 0.2rem;
  }
  .node {
    border: none;
    justify-content: flex-start;
    gap: 0.5rem;
    padding: 0.38rem 0.5rem;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
    font-size: 0.85rem;
    color: var(--side-text);
    width: 100%;
  }
  .node.strong {
    font-weight: 600;
    padding-left: 0.2rem;
  }
  .node.selected {
    background: var(--accent);
    color: var(--accent-text);
  }
  .node.selected .n {
    color: var(--accent-text);
  }
  .node.drop {
    outline: 2px dashed var(--accent);
    outline-offset: -2px;
  }
  .node.warn:not(.selected) {
    color: var(--warn);
  }
  .node.new {
    color: var(--side-muted);
    font-size: 0.8rem;
    padding-left: 0.5rem;
  }
  .chev {
    display: inline-flex;
    opacity: 0.7;
  }
  .label {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    font-size: 0.72rem;
    color: var(--side-muted);
  }
</style>
