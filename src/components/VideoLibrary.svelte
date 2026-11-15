<script lang="ts">
  // The Library workspace while the Video shelf is selected (decision 0035): the videos of the
  // selected folder on the left, the selected video's page on the right. Videos drag onto
  // folders (to move them) and onto scenes (to add them).
  import { FilePlus, FolderOpen, Star, TriangleAlert } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { formatBytes, formatDuration, run, ui } from "../lib/store.svelte";
  import { addVideoFiles, findVideos, folderOfVideo } from "../lib/video";
  import { videoSource } from "../lib/videoStream";
  import VideoPage from "./VideoPage.svelte";

  const lib = ui.library;
  const snapshot = $derived(ui.snapshot!);
  /** Only the videos that may not play (the "May not play" chip). */
  let mayNotPlay = $state(false);
  const all = $derived(findVideos(lib.folder, lib.search));
  const unplayable = $derived(all.filter((v) => v.info && !v.info.playsEverywhere));
  const list = $derived(mayNotPlay ? unplayable : all);
  // The chip goes away once nothing is left to convert.
  $effect(() => {
    if (unplayable.length === 0) mayNotPlay = false;
  });
  const missing = $derived(new Set(snapshot.missing));
  const selectedId = $derived(lib.selected.length === 1 && list.some((v) => v.id === lib.selected[0]) ? lib.selected[0] : null);
  const title = $derived(lib.folder ? lib.folder.split("/").pop()! : "Video");

  async function addFiles() {
    const ids = await addVideoFiles(lib.folder);
    if (ids.length) lib.selected = [ids[0]];
  }

  function dragStart(e: DragEvent, id: string) {
    ui.drag = { kind: "video", id };
    e.dataTransfer?.setData("text/plain", id);
    // WebKit refuses a drop unless "move" is allowed too (see soundDragStart).
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
  }
</script>

<section class="list">
  <div class="head">
    <div class="titles">
      <h2 class="display">{title}</h2>
      <span class="caps muted">Video · {list.length} video{list.length === 1 ? "" : "s"}{lib.search ? ` matching “${lib.search}”` : ""}</span>
    </div>
    {#if unplayable.length}
      <button class="chip warn" class:on={mayNotPlay} onclick={() => (mayNotPlay = !mayNotPlay)} title="Videos that may not play on this computer">
        <TriangleAlert size={12} />May not play · {unplayable.length}
      </button>
    {/if}
    <button class="icon framed" title="Add video files" onclick={addFiles}><FilePlus size={15} /></button>
    <button class="icon framed" title="Show this folder in the file manager" onclick={() => run(api.openAudioFolder("video", lib.folder))}>
      <FolderOpen size={15} />
    </button>
  </div>
  <div class="rows" role="listbox">
    {#each list as v (v.id)}
      <div
        class="row"
        class:selected={selectedId === v.id}
        role="option"
        aria-selected={selectedId === v.id}
        tabindex="0"
        draggable="true"
        onclick={() => (lib.selected = [v.id])}
        onkeydown={(e) => e.key === "Enter" && (lib.selected = [v.id])}
        ondragstart={(e) => dragStart(e, v.id)}
        ondragend={() => (ui.drag = null)}
      >
        <div class="thumb">
          {#if !missing.has(v.id)}
            <!-- A frame a few seconds in, as a thumbnail; only the metadata and that frame load. -->
            <video use:videoSource={{ video: v, at: 3, preview: true }} muted tabindex="-1"></video>
          {:else}
            <TriangleAlert size={14} />
          {/if}
        </div>
        <div class="text">
          <span class="name">{v.name}</span>
          <span class="meta" class:warn={v.info && !v.info.playsEverywhere}>
            {v.info ? formatDuration(v.info.durationMs) : formatBytes(v.file.size)}{v.info && !v.info.playsEverywhere ? " · may not play" : ""}{!lib.folder && folderOfVideo(v) ? ` · ${folderOfVideo(v)}` : ""}
          </span>
        </div>
        {#if v.favourite}<Star size={13} class="fav" fill="currentColor" />{/if}
      </div>
    {:else}
      <p class="empty">
        {lib.search ? "Nothing matches the search." : "No videos here yet."}
        {#if !lib.search}<button class="link" onclick={addFiles}><FilePlus size={12} />Add video files</button>{/if}
      </p>
    {/each}
  </div>
</section>

<section class="editor">
  {#if selectedId}
    {#key selectedId}<VideoPage id={selectedId} />{/key}
  {:else}
    <div class="placeholder">
      <h2 class="display">Choose a video</h2>
      <p class="muted">
        Videos play in the video window while their scene is the top-most one with videos. Their
        sound is never played. Add video files here, drop them onto the window, or put them in the
        project's video folder.
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
    border-bottom: 1px solid var(--rule);
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
  .framed {
    border: 1px solid var(--rule) !important;
  }
  .chip.warn {
    color: var(--warn);
    font-size: 0.7rem;
    white-space: nowrap;
  }
  .chip.warn.on {
    background: var(--warn);
    border-color: var(--warn);
    color: var(--accent-text);
  }
  .rows {
    flex: 1;
    overflow: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.45rem 1rem;
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
  .thumb {
    width: 4.5rem;
    height: 2.55rem;
    flex: none;
    border-radius: 2px;
    background: #2a2220;
    color: var(--warn);
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .thumb video {
    width: 100%;
    height: 100%;
    object-fit: cover;
    pointer-events: none;
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
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .meta {
    font-size: 0.7rem;
    color: var(--muted);
  }
  .meta.warn {
    color: var(--warn);
  }
  .row :global(.fav) {
    color: var(--accent);
    flex: none;
  }
  .empty {
    padding: 1rem;
    color: var(--muted);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }
  .link {
    border: none;
    padding: 0;
    color: var(--accent);
  }
  .editor {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .placeholder {
    margin: auto;
    max-width: 28rem;
    text-align: center;
    padding: 2rem;
  }
  .placeholder h2 {
    font-size: 2.2rem;
    margin-bottom: 0.6rem;
  }
</style>
