<script lang="ts">
  // The full-screen Library workspace (decision 0018): folder tree | sound list | editor.
  // The tapestry keeps playing in the mixer bar; Esc or the breadcrumbs go back.
  //
  // The list only draws the rows that are on screen ("virtual scrolling"), so a library with
  // thousands of sounds scrolls as smoothly as one with ten.
  import { ArrowDownAZ, FolderOpen, Play, Square, Star, TriangleAlert } from "@lucide/svelte";
  import { onDestroy } from "svelte";
  import { api } from "../lib/api";
  import { stopPreviewOnLeave } from "../lib/actions";
  import { findSounds, soundDragStart, tapestrySoundIds, type Filter } from "../lib/library";
  import { durationOf, formatBytes, formatDuration, folderOf, run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Sound } from "../lib/types";
  import BulkEditor from "./BulkEditor.svelte";
  import CropEditor from "./CropEditor.svelte";
  import LibraryTree from "./LibraryTree.svelte";
  import SoundEditor from "./SoundEditor.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  const lib = ui.library;
  // Leaving the Library stops whatever was being previewed there.
  onDestroy(() => stopPreviewOnLeave());

  let sort = $state<"name" | "recent" | "size" | "loudness">("name");

  const list = $derived.by(() => {
    const found = findSounds(project, snapshot.missing, {
      filter: (lib.category ? "all" : lib.smart) as Filter,
      category: lib.category,
      folder: lib.category ? lib.folder : null,
      deep: true,
      search: lib.search,
    });
    if (sort === "size") return [...found].sort((a, b) => b.file.size - a.file.size);
    if (sort === "loudness") return [...found].sort((a, b) => (b.file.loudness?.lufs ?? -99) - (a.file.loudness?.lufs ?? -99));
    if (sort === "recent") return [...found].sort((a, b) => (b.lastUsed ?? "").localeCompare(a.lastUsed ?? ""));
    return found;
  });

  const title = $derived.by(() => {
    if (lib.category) {
      if (lib.folder) return lib.folder.split("/").pop()!;
      return CATEGORIES.find((c) => c.id === lib.category)!.label;
    }
    return { all: "All sounds", favourites: "Favourites", recent: "Recent", unused: "Unused", missing: "Missing files" }[lib.smart];
  });
  const subtitle = $derived(
    `${lib.category ? `${CATEGORIES.find((c) => c.id === lib.category)!.label} · ` : ""}${list.length} sound${list.length === 1 ? "" : "s"}${lib.search ? ` matching “${lib.search}”` : ""}`,
  );
  const live = $derived(tapestrySoundIds(project));
  const missing = $derived(new Set(snapshot.missing));
  const selected = $derived(new Set(lib.selected));
  const selectedSounds = $derived(lib.selected.map((id) => project.sounds.find((s) => s.id === id)).filter(Boolean) as Sound[]);

  // Drop selections of sounds that no longer exist (removed, or filtered away by a move).
  $effect(() => {
    const ids = new Set(project.sounds.map((s) => s.id));
    if (lib.selected.some((id) => !ids.has(id))) lib.selected = lib.selected.filter((id) => ids.has(id));
  });

  // ---- Selection: click, Ctrl/⌘-click to add, Shift-click for a range ----
  let anchor: string | null = null;
  function select(e: MouseEvent, sound: Sound) {
    lib.cropping = false;
    if (e.shiftKey && anchor) {
      const a = list.findIndex((s) => s.id === anchor);
      const b = list.findIndex((s) => s.id === sound.id);
      if (a >= 0 && b >= 0) {
        const [from, to] = a < b ? [a, b] : [b, a];
        lib.selected = list.slice(from, to + 1).map((s) => s.id);
        return;
      }
    }
    if (e.ctrlKey || e.metaKey) {
      lib.selected = selected.has(sound.id) ? lib.selected.filter((id) => id !== sound.id) : [...lib.selected, sound.id];
    } else {
      lib.selected = [sound.id];
    }
    anchor = sound.id;
  }

  function selectAll(on: boolean) {
    lib.selected = on ? list.map((s) => s.id) : [];
  }

  // ---- Virtual scrolling ----
  const ROW = 54;
  let scrollTop = $state(0);
  let viewport = $state(600);
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - 6));
  const last = $derived(Math.min(list.length, Math.ceil((scrollTop + viewport) / ROW) + 6));
  const visible = $derived(list.slice(first, last));

  function meta(s: Sound): string {
    const length = durationOf(s);
    const parts = [length !== null ? `${formatDuration(length)} · ${formatBytes(s.file.size)}` : formatBytes(s.file.size)];
    if (s.file.loudness) parts.push(`${s.file.loudness.lufs.toFixed(1)} LUFS`);
    if (!lib.category) parts.push(`${CATEGORIES.find((c) => c.id === s.category)!.label}${folderOf(s) ? ` › ${folderOf(s)}` : ""}`);
    return parts.join(" · ");
  }
</script>

<LibraryTree />

<section class="list">
  <div class="head">
    <div class="titles">
      <h2 class="display">{title}</h2>
      <span class="caps muted">{subtitle}</span>
    </div>
    <label class="sort" title="Sort">
      <ArrowDownAZ size={14} />
      <select bind:value={sort}>
        <option value="name">Name</option>
        <option value="recent">Recently used</option>
        <option value="size">Size</option>
        <option value="loudness">Loudness</option>
      </select>
    </label>
    {#if lib.category}
      <button class="icon framed" title="Show this folder in the file manager" onclick={() => run(api.openAudioFolder(lib.category, lib.folder))}>
        <FolderOpen size={15} />
      </button>
    {/if}
  </div>
  <div class="selection">
    <input
      type="checkbox"
      checked={list.length > 0 && lib.selected.length === list.length}
      indeterminate={lib.selected.length > 0 && lib.selected.length < list.length}
      onchange={(e) => selectAll(e.currentTarget.checked)}
      aria-label="Select all"
    />
    <span>{lib.selected.length > 1 ? `${lib.selected.length} selected` : "Shift / Ctrl-click to edit several at once"}</span>
  </div>
  <div class="rows" bind:clientHeight={viewport} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)} role="listbox" aria-multiselectable="true">
    <div class="spacer" style:height="{list.length * ROW}px">
      {#each visible as s, i (s.id)}
        {@const previewing = snapshot.playback.preview === s.id}
        <div
          class="row"
          class:selected={selected.has(s.id)}
          style:top="{(first + i) * ROW}px"
          role="option"
          aria-selected={selected.has(s.id)}
          tabindex="0"
          draggable="true"
          onclick={(e) => select(e, s)}
          onkeydown={(e) => e.key === "Enter" && select(e as unknown as MouseEvent, s)}
          ondragstart={(e) => soundDragStart(e, s.id, (id) => (ui.drag = { kind: "sound", id }))}
          ondragend={() => (ui.drag = null)}
        >
          <input type="checkbox" checked={selected.has(s.id)} onclick={(e) => { e.stopPropagation(); select({ ctrlKey: true, metaKey: false, shiftKey: e.shiftKey } as MouseEvent, s); }} aria-label="Select" />
          <button
            class="play"
            title={previewing ? "Stop" : "Preview"}
            disabled={missing.has(s.id)}
            onclick={(e) => {
              e.stopPropagation();
              run(api.previewSound(s.id));
            }}
          >
            {#if previewing}<Square size={11} />{:else}<Play size={11} />{/if}
          </button>
          <div class="text">
            <span class="name">
              {#if missing.has(s.id)}<TriangleAlert size={12} />{/if}
              {s.name}
              {#if live.has(s.id)}<i class="live" title="In the tapestry"></i>{/if}
            </span>
            <span class="meta">{meta(s)}{#each s.tags ?? [] as t (t)}<span class="tag">#{t}</span>{/each}</span>
          </div>
          <Star size={14} class={s.favourite ? "fav on" : "fav"} fill={s.favourite ? "currentColor" : "none"} />
        </div>
      {/each}
    </div>
    {#if list.length === 0}
      <p class="empty">
        {lib.search ? "Nothing matches the search." : lib.smart === "missing" && !lib.category ? "No missing files." : "No sounds here yet. Add files or download from a URL."}
      </p>
    {/if}
  </div>
</section>

<section class="editor">
  {#if selectedSounds.length === 1}
    {#if lib.cropping}
      {#key selectedSounds[0].id}<CropEditor sound={selectedSounds[0]} />{/key}
    {:else}
      {#key selectedSounds[0].id}<SoundEditor id={selectedSounds[0].id} />{/key}
    {/if}
  {:else if selectedSounds.length > 1}
    <BulkEditor sounds={selectedSounds} />
  {:else}
    <div class="placeholder">
      <h2 class="display">Choose a sound</h2>
      <p class="muted">
        Select a sound to edit its name, tags, levels and fades, crop it, or see where it's used.
        Select several with Shift or Ctrl to change them together. Drag sounds onto a folder on
        the left to move them.
      </p>
    </div>
  {/if}
</section>

<style>
  .list {
    width: 24rem;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--fx-bg);
    border-right: 1px solid var(--rule);
  }
  .head {
    display: flex;
    align-items: flex-end;
    gap: 0.5rem;
    padding: 1.1rem 1rem 0.7rem;
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  h2 {
    font-size: 1.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .titles .caps {
    font-size: 0.58rem;
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    color: var(--muted);
  }
  .sort select {
    padding: 0.25rem 0.4rem;
    font-size: 0.8rem;
  }
  .framed {
    border: 1px solid var(--rule) !important;
  }
  .selection {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.45rem 1rem;
    border-top: 1px solid var(--rule);
    border-bottom: 1px solid var(--rule);
    font-size: 0.75rem;
    color: var(--muted);
  }
  .rows {
    flex: 1;
    overflow: auto;
    position: relative;
  }
  .spacer {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 54px;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0 1rem;
    border-bottom: 1px solid color-mix(in srgb, var(--rule) 60%, transparent);
    cursor: default;
  }
  .row:hover {
    background: color-mix(in srgb, var(--text) 5%, transparent);
  }
  .row.selected {
    background: var(--surface);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .play {
    width: 1.6rem;
    height: 1.6rem;
    border-radius: 50%;
    padding: 0;
    justify-content: center;
    border-color: var(--rule);
    flex: none;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .name {
    font-weight: 600;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name :global(svg) {
    color: var(--warn);
    flex: none;
  }
  .live {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: var(--accent);
    flex: none;
  }
  .meta {
    font-size: 0.7rem;
    color: var(--muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .tag {
    margin-left: 0.4rem;
  }
  .row :global(.fav) {
    color: var(--rule);
    flex: none;
  }
  .row :global(.fav.on) {
    color: var(--accent);
  }
  .empty {
    padding: 1rem;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .editor {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
    min-height: 0;
  }
  .placeholder {
    margin: auto;
    max-width: 26rem;
    text-align: center;
    padding: 2rem;
  }
  .placeholder h2 {
    font-size: 2rem;
  }
  .placeholder p {
    line-height: 1.5;
  }
</style>
