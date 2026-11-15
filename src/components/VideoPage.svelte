<script lang="ts">
  // One video in the Library workspace (decision 0035, design screen 6): a muted preview, name,
  // folder, tags, format, where it's used, its file and source. Unlike the sound editor there
  // is no draft: each change is saved straight away, since there are no levels to try out.
  import { CircleCheck, ExternalLink, FileVideo, FolderOpen, FolderSearch, Play, RefreshCw, Scissors, Scroll, Star, Trash2, TriangleAlert, X } from "@lucide/svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { askText, formatBytes, formatDuration, run, ui } from "../lib/store.svelte";
  import type { Video, VideoKind } from "../lib/types";
  import { describeVideo, folderOfVideo, kindOf, moveVideos, relinkVideo, removeVideo, scenesUsingVideo, shelfOfKind, videoById } from "../lib/video";
  import { videoSource } from "../lib/videoStream";
  import TimeInput from "./TimeInput.svelte";

  let { id }: { id: string } = $props();

  const video = $derived(videoById(id));
  const missing = $derived(ui.snapshot!.missing.includes(id));
  const folders = $derived(video && kindOf(video) === "effect" ? ui.snapshot!.videoFolders.effect : ui.snapshot!.videoFolders.ambience);
  const usedIn = $derived(scenesUsingVideo(id));
  const conversion = $derived(ui.conversions.find((c) => c.id === id));
  const busy = $derived(!!conversion && (conversion.status === "running" || conversion.status === "queued"));

  // ---- Trimming: keep only a part, re-encoded in the background so the cut is exact ----
  let playerEl = $state<HTMLVideoElement>();
  let trimming = $state(false);
  let trimStart = $state(0);
  let trimEnd = $state(0);
  let trimAsCopy = $state(true);
  let trimError = $state("");
  /** Playing the selection from the panel: stop at its end. */
  let previewingTrim = false;
  const lengthMs = $derived(video?.info?.durationMs ?? (playerEl ? playerEl.duration * 1000 : 0));
  const here = () => Math.round((playerEl?.currentTime ?? 0) * 1000);

  function openTrim() {
    trimStart = 0;
    trimEnd = Math.round(lengthMs);
    trimError = "";
    trimming = true;
  }

  function previewTrim() {
    if (!playerEl) return;
    playerEl.currentTime = trimStart / 1000;
    previewingTrim = true;
    playerEl.play().catch(() => {});
  }

  function stopAtTrimEnd() {
    if (previewingTrim && playerEl && playerEl.currentTime * 1000 >= trimEnd) {
      playerEl.pause();
      previewingTrim = false;
    }
  }

  async function trim() {
    trimError = "";
    try {
      await api.trimVideo(id, trimStart, trimEnd, trimAsCopy);
      trimming = false;
    } catch (e) {
      trimError = String(e) === "TOOLS_MISSING" ? "Trimming needs ffmpeg: install the download tools in Preferences › Projects & storage." : String(e);
    }
  }
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
    if (!video) return;
    if (value === "__new__") {
      const name = await askText("New folder", "Name");
      if (!name) return;
      await run(api.createFolder(shelfOfKind(kindOf(video)), name));
      value = name;
    }
    await moveVideos([id], kindOf(video), value);
  }

  /** Ambience video ↔ video effect: the file moves to the other kind's folder (top level). */
  async function changeKind(kind: VideoKind) {
    if (video && kind !== kindOf(video)) await moveVideos([id], kind, "");
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
          bind:this={playerEl}
          use:videoSource={{ video }}
          ontimeupdate={stopAtTrimEnd}
          onpause={() => (previewingTrim = false)}
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

    {#if !missing}
      <div class="trim-bar">
        {#if trimming}
          <div class="trim">
            <div class="trim-times">
              <span class="caps">Keep from</span>
              <TimeInput label="Start" value={trimStart} onchange={(ms) => (trimStart = ms)} />
              <button class="small" title="Take the preview's position" onclick={() => (trimStart = here())}>Set to the preview's position</button>
              <span class="caps">to</span>
              <TimeInput label="End" value={trimEnd} onchange={(ms) => (trimEnd = ms)} />
              <button class="small" title="Take the preview's position" onclick={() => (trimEnd = here())}>Set to the preview's position</button>
            </div>
            <div class="trim-times">
              <span class="muted">Keeps {formatDuration(Math.max(0, trimEnd - trimStart))}</span>
              <button class="small" onclick={previewTrim} disabled={trimEnd <= trimStart}><Play size={12} />Preview the part</button>
              <span class="grow"></span>
              <label class="radio"><input type="radio" checked={trimAsCopy} onchange={() => (trimAsCopy = true)} />Save as a copy</label>
              <label class="radio"><input type="radio" checked={!trimAsCopy} onchange={() => (trimAsCopy = false)} />Replace the original (it goes to the trash)</label>
            </div>
            {#if trimError}<p class="error">{trimError}</p>{/if}
            <div class="trim-times">
              <span class="muted small grow">Re-encoded in the background (as WebM), so the cut is exact; a short part is quick.</span>
              <button class="ghost" onclick={() => (trimming = false)}>Cancel</button>
              <button class="primary" onclick={trim} disabled={trimEnd <= trimStart + 100}><Scissors size={13} />Trim</button>
            </div>
          </div>
        {:else if busy && conversion}
          <span class="muted">{conversion.trimming ? "Trimming" : "Converting"} · {Math.round(conversion.progress * 100)}%</span>
          <button class="ghost small" onclick={() => run(api.cancelConversion(id))}>Cancel</button>
        {:else}
          <button onclick={openTrim} disabled={!lengthMs}><Scissors size={13} />Trim…</button>
        {/if}
      </div>
    {/if}
    <section>
      <div class="grid">
        <label class="field">
          <span class="caps">Type</span>
          <select value={kindOf(video)} onchange={(e) => changeKind(e.currentTarget.value as VideoKind)} disabled={missing}>
            <option value="ambience">Ambience video</option>
            <option value="effect">Video effect</option>
          </select>
        </label>
        <label class="field">
          <span class="caps">Folder</span>
          <select value={folderOfVideo(video)} onchange={(e) => changeFolder(e.currentTarget.value)} disabled={missing}>
            <option value="">({kindOf(video) === "effect" ? "Video effects" : "Ambience videos"} folder)</option>
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
            {#if busy && conversion}
              <span class="muted">{conversion.trimming ? "trimming" : "converting"} · {Math.round(conversion.progress * 100)}%</span>
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
                prefill: { url: source.url, name: video.name, category: shelfOfKind(kindOf(video)), folder: folderOfVideo(video), range: source.range ?? null, replace: id },
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
  .trim-bar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex: none;
  }
  .trim {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.8rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
  }
  .trim-times {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .trim .small {
    font-size: 0.62rem;
  }
  .trim .radio {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.8rem;
  }
  .error {
    color: var(--danger);
    margin: 0;
  }
  .muted.small {
    font-size: 0.75rem;
  }
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
    grid-template-columns: 1fr 1fr 1.4fr;
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
