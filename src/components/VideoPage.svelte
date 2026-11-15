<script lang="ts">
  // One video in the Library workspace (decision 0035, design screen 6): a muted preview, name,
  // folder, tags, format, where it's used, its file and source. Unlike the sound editor there
  // is no draft: each change is saved straight away, since there are no levels to try out.
  import { CircleCheck, ExternalLink, FileVideo, FolderOpen, FolderSearch, RefreshCw, Scroll, Star, Trash2, TriangleAlert, X } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { askText, formatBytes, formatDuration, run, ui } from "../lib/store.svelte";
  import type { Video } from "../lib/types";
  import { describeVideo, folderOfVideo, relinkVideo, removeVideo, scenesUsingVideo, videoById } from "../lib/video";
  import { videoSource } from "../lib/videoStream";

  let { id }: { id: string } = $props();

  const video = $derived(videoById(id));
  const missing = $derived(ui.snapshot!.missing.includes(id));
  const folders = $derived(ui.snapshot!.videoFolders);
  const usedIn = $derived(scenesUsingVideo(id));
  const conversion = $derived(ui.conversions.find((c) => c.id === id));
  let newTag = $state("");

  function save(change: Partial<Video>) {
    if (video) run(api.updateVideos([{ ...$state.snapshot(video), ...change }]));
  }

  function rename(e: Event) {
    const value = (e.currentTarget as HTMLInputElement).value.trim();
    if (video && value && value !== video.name) save({ name: value });
  }

  function addTag() {
    const t = newTag.trim().replace(/^#/, "").toLowerCase();
    newTag = "";
    if (video && t && !(video.tags ?? []).includes(t)) save({ tags: [...(video.tags ?? []), t] });
  }

  async function changeFolder(value: string) {
    if (value === "__new__") {
      const name = await askText("New folder", "Name");
      if (!name) return;
      await run(api.createFolder("video", name));
      value = name;
    }
    await run(api.moveVideos([id], value));
  }
</script>

{#if video}
  <div class="page">
    <div class="head">
      <input class="name display" value={video.name} onchange={rename} onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()} aria-label="Video name" />
      <button class="icon" title={video.favourite ? "Remove from favourites" : "Add to favourites"} onclick={() => save({ favourite: !video.favourite })}>
        <Star size={18} class={video.favourite ? "fav on" : "fav"} fill={video.favourite ? "currentColor" : "none"} />
      </button>
      <button class="icon" title="Remove from the project…" onclick={() => removeVideo(id)}><Trash2 size={17} /></button>
    </div>

    <div class="player">
      {#if !missing}
        <!-- Muted: a video's sound is never played (decision 0035). -->
        <!-- svelte-ignore a11y_media_has_caption -->
        <!-- Sized by its own proportions (known from the file before it loads), within the
             page's width and 42% of the window's height. -->
        <video
          use:videoSource={{ video }}
          controls
          muted
          style:aspect-ratio={video.info ? `${video.info.width} / ${video.info.height}` : "16 / 9"}
        ></video>
      {:else}
        <div class="no-preview muted">
          <TriangleAlert size={20} />
          {missing ? "The file is missing." : "No preview."}
        </div>
      {/if}
    </div>

    <section>
      <div class="grid">
        <label class="field">
          <span class="caps">Folder</span>
          <select value={folderOfVideo(video)} onchange={(e) => changeFolder(e.currentTarget.value)} disabled={missing}>
            <option value="">(Video folder)</option>
            {#each folders as f (f)}<option value={f}>{f}</option>{/each}
            <option value="__new__">+ New folder…</option>
          </select>
        </label>
        <div class="field">
          <span class="caps">Tags</span>
          <div class="tags">
            {#each video.tags ?? [] as t (t)}
              <span class="chip">#{t}<button class="icon x" title="Remove" onclick={() => save({ tags: (video.tags ?? []).filter((x) => x !== t) })}><X size={11} /></button></span>
            {/each}
            <input
              class="tag-input"
              bind:value={newTag}
              placeholder="+ tag"
              onkeydown={(e) => (e.key === "Enter" || e.key === ",") && (e.preventDefault(), addTag())}
              onblur={addTag}
            />
          </div>
        </div>
      </div>
    </section>

    <section>
      <div class="section-head"><span class="caps">Format</span></div>
      <div class="line">
        {#if video.info}
          {#if video.info.playsEverywhere}
            <span class="ok"><CircleCheck size={15} />{describeVideo(video, formatDuration)} · {formatBytes(video.file.size)} · plays here</span>
          {:else}
            <span class="warn"><TriangleAlert size={15} />{describeVideo(video, formatDuration)} · {formatBytes(video.file.size)} · may not play on every computer</span>
            {#if conversion && (conversion.status === "running" || conversion.status === "queued")}
              <span class="muted">converting · {Math.round(conversion.progress * 100)}%</span>
            {:else}
              <button onclick={() => (ui.convertOffer = { ids: [id], origin: "library" })}><RefreshCw size={13} />Convert…</button>
            {/if}
          {/if}
        {:else}
          <span class="muted">{missing ? "Can't be looked at while the file is missing." : `${formatBytes(video.file.size)} · not looked at yet; it happens in the background.`}</span>
        {/if}
      </div>
    </section>

    <section>
      <div class="section-head"><span class="caps">Used in</span></div>
      <div class="line wrap">
        {#each usedIn as s (s.id)}
          <span class="chip"><Scroll size={12} />{s.name}{s.videoEffects?.includes(id) && !s.video?.videos.includes(id) ? " (video effect)" : ""}</span>
        {:else}
          <span class="muted">No scene shows it yet. Add it in a scene's Video section.</span>
        {/each}
      </div>
    </section>

    <section>
      <div class="section-head"><span class="caps">File</span></div>
      <div class="line">
        <FileVideo size={16} />
        <code class="grow">{video.file.path}</code>
        <button onclick={() => run(api.revealVideo(id))} disabled={missing}><FolderSearch size={13} />Show in folder</button>
        <button onclick={() => relinkVideo(id)}><FolderOpen size={13} />Replace file…</button>
      </div>
    </section>

    {#if video.source}
      {@const source = video.source}
      <section>
        <div class="section-head"><span class="caps">Source</span></div>
        <div class="line">
          <span class="grow"><strong>“{source.title ?? video.name}”</strong> <span class="muted">· {source.site ?? ""} · downloaded {new Date(source.downloadedAt).toLocaleDateString()}</span></span>
          <button onclick={() => openUrl(source.url)}><ExternalLink size={13} />Open page</button>
          <button
            onclick={() =>
              (ui.editing = {
                kind: "addUrl",
                prefill: { url: source.url, name: video.name, category: "video", folder: folderOfVideo(video), range: source.range ?? null, replace: id },
              })}
          >
            <RefreshCw size={13} />Download again
          </button>
        </div>
      </section>
    {/if}
  </div>
{/if}

<style>
  .page {
    flex: 1;
    overflow: auto;
    padding: 1.3rem 2rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-size: 2.3rem;
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
  .head :global(.fav) {
    color: var(--rule);
  }
  .head :global(.fav.on) {
    color: var(--accent);
  }
  /* The box follows the video: no fixed proportions of its own, so the picture is never
     stretched or cropped. */
  .player {
    background: #000;
    border-radius: var(--radius);
    overflow: hidden;
    display: flex;
    justify-content: center;
    /* Not "stretch": the video's height comes from its own proportions. */
    align-items: flex-start;
    min-height: 6rem;
    /* The page is a scrolling column: without this, a long page squeezes the player (and
       crops the video) instead of scrolling. */
    flex: none;
  }
  /* Width and height both auto, with only maximums: the browser keeps the video's own
     proportions (its file's, or `aspect-ratio` until it loads) while fitting both limits. */
  .player video {
    display: block;
    width: auto;
    height: auto;
    max-width: 100%;
    max-height: 42vh;
  }
  .no-preview {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    color: #bbb;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: 0.8rem;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    align-items: center;
    min-height: 2.3rem;
  }
  .x {
    padding: 0.05rem;
  }
  .tag-input {
    width: 6rem;
    padding: 0.25rem 0.5rem;
    border-radius: 999px;
    border-style: dashed;
    font-size: 0.8rem;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 0.7rem;
  }
  .line.wrap {
    flex-wrap: wrap;
  }
  .ok,
  .warn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  .ok {
    color: var(--ok);
  }
  .warn {
    color: var(--warn);
  }
  code {
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
</style>
