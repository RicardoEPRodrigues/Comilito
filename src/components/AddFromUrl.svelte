<script lang="ts">
  // "Add from a URL" (decision 0019): paste a video page or a direct link to an audio file,
  // check what it is, choose the part, name, category and folder, then download. The
  // download runs in the background (see Downloads in the Library). Nothing is streamed.
  // A page can also be downloaded as a video, the picture only (decision 0035, design 4).
  import { AudioLines, CircleCheck, Film, Globe, Info, LoaderCircle, Link, Scale, Download as DownloadIcon } from "@lucide/svelte";
  import { videoEnabled } from "../lib/video";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { askText, formatBytes, formatDuration, refreshTools, run, ui, type UrlPrefill } from "../lib/store.svelte";
  import { CATEGORIES, type Category, type Probe, type Shelf } from "../lib/types";
  import Modal from "./Modal.svelte";
  import TimeInput from "./TimeInput.svelte";
  import ToolsInstall from "./ToolsInstall.svelte";

  // `target`: the folder right-clicked in the Library ("Add from URL…" there).
  let {
    onclose,
    prefill,
    target,
  }: { onclose: () => void; prefill?: UrlPrefill; target?: { category: Shelf; folder: string } } = $props();

  // svelte-ignore state_referenced_locally
  let url = $state(prefill?.url ?? "");
  let probing = $state(false);
  let probe = $state<Probe | null>(null);
  let error = $state("");
  let needTools = $state(false);
  // svelte-ignore state_referenced_locally
  let partial = $state(!!prefill?.range);
  // svelte-ignore state_referenced_locally
  let fromMs = $state(prefill?.range?.startMs ?? 0);
  // svelte-ignore state_referenced_locally
  let toMs = $state(prefill?.range?.endMs ?? 10 * 60 * 1000);
  let name = $state("");
  // svelte-ignore state_referenced_locally
  const startShelf = prefill?.category ?? target?.category ?? ui.library.category ?? "ambience";
  let category = $state<Category>(startShelf === "video" ? "ambience" : startShelf);
  /** Download as a video (the picture only) instead of audio. */
  let asVideo = $state(startShelf === "video" && videoEnabled());
  let height = $state(ui.prefs?.video.downloadHeight ?? 1080);
  const videoOption = $derived(probe?.videoOptions.find((o) => o.maxHeight === height) ?? probe?.videoOptions.at(-1));
  const canVideo = $derived(videoEnabled() && !!probe && (probe.isVideo || probe.videoOptions.length > 0));
  // svelte-ignore state_referenced_locally
  let folder = $state(prefill?.folder ?? target?.folder ?? (ui.library.category && ui.library.folder ? ui.library.folder : ""));
  let updateAvailable = $state<boolean | null>(null);

  onMount(() => {
    refreshTools();
    api.toolsUpdateAvailable().then((v) => (updateAvailable = v)).catch(() => {});
    // "Download again": look the link up straight away, keeping the original settings.
    if (prefill) check();
  });

  async function check() {
    if (!url.trim()) return;
    probing = true;
    error = "";
    probe = null;
    needTools = false;
    try {
      probe = await api.probeUrl(url.trim());
      // A link to a video file can only be a video; a page without video formats only audio.
      if (probe.isVideo) asVideo = true;
      else if (!probe.videoOptions.length) asVideo = false;
      if (prefill) {
        name = prefill.name;
      } else {
        name = probe.title;
        if (probe.durationMs && probe.durationMs > 20 * 60 * 1000) {
          // Long ambience videos: suggest a 10-minute part.
          partial = true;
          fromMs = 0;
          toMs = 10 * 60 * 1000;
        } else {
          partial = false;
          toMs = probe.durationMs ?? 10 * 60 * 1000;
        }
      }
    } catch (e) {
      const message = String(e);
      if (message.includes("TOOLS_MISSING")) needTools = true;
      else error = message;
    } finally {
      probing = false;
    }
  }

  const range = $derived(partial && toMs > fromMs ? { startMs: fromMs, endMs: toMs } : null);

  async function newFolder() {
    const created = await askText("New folder", "Name");
    if (!created) return;
    if (await run(api.createFolder(asVideo ? "video" : category, created).then(() => true))) folder = created;
  }
  /** The whole download's size: the audio, or the chosen video format. */
  const fullSize = $derived(asVideo && probe && !probe.isVideo ? (videoOption?.size ?? null) : (probe?.size ?? null));
  const estimate = $derived.by(() => {
    if (!fullSize) return null;
    if (!range || !probe?.durationMs) return fullSize;
    return Math.round((fullSize * (range.endMs - range.startMs)) / probe.durationMs);
  });
  const folders = $derived(asVideo ? (ui.snapshot?.videoFolders ?? []) : (ui.snapshot?.folders[category] ?? []));
  const canDownload = $derived(!!probe && name.trim() !== "" && (!partial || range !== null) && (probe.kind === "direct" || !!ui.tools?.ffmpeg));

  async function download() {
    if (!probe) return;
    const id = await run(
      api.startDownload({
        url: probe.url,
        kind: probe.kind,
        title: probe.title,
        site: probe.site,
        ext: probe.ext,
        category,
        folder,
        name: name.trim(),
        // Cutting a part only works for pages (yt-dlp); a video file link is taken whole.
        range: asVideo && probe.isVideo ? null : range,
        replace: prefill?.replace ?? null,
        videoHeight: asVideo ? (probe.isVideo ? 0 : height) : null,
      }),
    );
    if (id) onclose();
  }
</script>

<Modal title={videoEnabled() ? "Add from a URL" : "Add audio from a URL"} {onclose}>
  <div class="url" class:ok={!!probe}>
    <Globe size={16} />
    <!-- svelte-ignore a11y_autofocus -->
    <input bind:value={url} placeholder="https://www.youtube.com/watch?v=… or a link to an .mp3/.ogg/.m4a file" autofocus
      onkeydown={(e) => e.key === "Enter" && check()} onpaste={() => setTimeout(check, 0)} />
    <button onclick={check} disabled={probing || !url.trim()}>{#if probing}<LoaderCircle size={13} class="spin" />{:else}<Link size={13} />{/if}Check</button>
  </div>
  <p class="muted small">
    Video pages (YouTube and many other sites) or a direct link to an audio file. The audio is
    downloaded once and saved into your project; nothing is streamed.
  </p>

  {#if needTools}
    <ToolsInstall reason="Downloading from video sites needs yt-dlp and ffmpeg." onready={check} />
  {/if}
  {#if error}<p class="error">{error}</p>{/if}

  {#if probe}
    <div class="detected">
      {#if probe.thumbnail}<img src={probe.thumbnail} alt="" />{/if}
      <div class="text">
        <strong>{probe.title}</strong>
        <span class="muted small">
          {probe.site}{probe.durationMs ? ` · ${formatDuration(probe.durationMs)}` : ""}{probe.quality ? ` · ${probe.quality}` : ""}
        </span>
      </div>
    </div>

    {#if canVideo}
      <div class="field">
        <span class="caps">Download as</span>
        <div class="segmented" role="radiogroup">
          <button class:on={!asVideo} disabled={probe.isVideo} onclick={() => ((asVideo = false), (folder = ""))} role="radio" aria-checked={!asVideo}><AudioLines size={14} />Audio</button>
          <button class:on={asVideo} onclick={() => ((asVideo = true), (folder = ""))} role="radio" aria-checked={asVideo}><Film size={14} />Video</button>
        </div>
        {#if asVideo}
          <span class="muted small">The picture only, without its sound: Comilito never plays a video's sound. Smaller files too.</span>
        {/if}
      </div>
      {#if asVideo && !probe.isVideo && videoOption}
        <div class="quality">
          <label class="field">
            <span class="caps">Resolution</span>
            <select bind:value={height}>
              <option value={720}>Up to 720p</option>
              <option value={1080}>Up to 1080p</option>
              <option value={1440}>Up to 1440p</option>
              <option value={0}>Best available</option>
            </select>
          </label>
          <div class="field grow">
            <span class="caps">Format</span>
            {#if videoOption.playsEverywhere}
              <span class="badge ok"><CircleCheck size={14} />{videoOption.ext.toUpperCase()} ({videoOption.codec.toUpperCase()}) · {videoOption.height}p: plays here</span>
            {:else}
              <span class="badge warn"><Info size={14} />Only {videoOption.ext.toUpperCase()} ({videoOption.codec.toUpperCase()}): converted to WebM after downloading, which takes a while</span>
            {/if}
          </div>
        </div>
      {/if}
    {/if}

    <div class="field" class:hidden={asVideo && probe.isVideo}>
      <span class="caps">What to download</span>
      <label class="radio"><input type="radio" bind:group={partial} value={false} />Everything {#if fullSize}<span class="muted small">≈ {formatBytes(fullSize)}</span>{/if}</label>
      <div class="radio">
        <label class="radio"><input type="radio" bind:group={partial} value={true} />Only from</label>
        <TimeInput label="From" value={fromMs} onchange={(ms) => ((fromMs = ms), (partial = true))} />
        to
        <TimeInput label="To" value={toMs} onchange={(ms) => ((toMs = ms), (partial = true))} />
      </div>
      {#if partial && estimate}<span class="muted small">≈ {formatBytes(estimate)} · long ambience loops rarely need hours</span>{/if}
      {#if partial && !range}<span class="error small">The end must be after the start.</span>{/if}
    </div>

    <div class="save-to">
      <label class="field grow"><span class="caps">Name</span><input bind:value={name} /></label>
      {#if !asVideo}
        <label class="field">
          <span class="caps">Category</span>
          <select bind:value={category} onchange={() => (folder = "")}>
            {#each CATEGORIES as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
          </select>
        </label>
      {/if}
      <label class="field">
        <span class="caps">Folder</span>
        <select
          value={folder}
          onchange={(e) => {
            if (e.currentTarget.value === "__new__") {
              e.currentTarget.value = folder;
              newFolder();
            } else folder = e.currentTarget.value;
          }}
        >
          <option value="">{asVideo ? "(Video folder)" : "(category folder)"}</option>
          {#each folders as f (f)}<option value={f}>{f}</option>{/each}
          <option value="__new__">+ New folder…</option>
        </select>
      </label>
    </div>

    {#if probe.kind === "page" && ui.tools && !ui.tools.ffmpeg}
      <ToolsInstall reason="This download needs yt-dlp and ffmpeg." />
    {/if}

    <div class="rights">
      <Scale size={15} />
      <span>
        Only download what you have the right to use. Many sites' terms don't allow downloading;
        respecting them and the creators' rights is up to you.
      </span>
    </div>
  {/if}

  {#snippet footer()}
    <span class="muted small tools">
      {#if ui.tools?.ytDlp}via yt-dlp {ui.tools.ytDlp}{#if updateAvailable} · <button class="link" onclick={() => run(api.installTools().then((t) => (ui.tools = t)))}>update available</button>{/if}{/if}
    </span>
    <button class="ghost" onclick={onclose}>Cancel</button>
    <button class="primary" onclick={download} disabled={!canDownload}><DownloadIcon size={14} />Download</button>
  {/snippet}
</Modal>

<style>
  .segmented {
    display: inline-flex;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    overflow: hidden;
    align-self: flex-start;
  }
  .segmented button {
    border: none;
    border-radius: 0;
    background: var(--surface);
    padding: 0.4rem 0.9rem;
    gap: 0.4rem;
  }
  .segmented button.on {
    background: var(--accent);
    color: var(--accent-text);
  }
  .quality {
    display: flex;
    gap: 0.8rem;
    align-items: flex-end;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius);
    font-size: 0.8rem;
  }
  .badge.ok {
    color: var(--ok);
    background: color-mix(in srgb, var(--ok) 10%, transparent);
  }
  .badge.warn {
    color: var(--warn);
    background: color-mix(in srgb, var(--warn) 10%, transparent);
  }
  .hidden {
    display: none;
  }
  .url {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.2rem 0.3rem 0.2rem 0.7rem;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--muted);
  }
  .url:focus-within,
  .url.ok {
    border-color: var(--accent);
  }
  .url input {
    flex: 1;
    border: none;
    background: transparent;
    outline: none;
  }
  .small {
    font-size: 0.8rem;
  }
  p {
    margin: 0;
    line-height: 1.45;
  }
  .error {
    color: var(--danger);
  }
  .detected {
    display: flex;
    align-items: center;
    gap: 0.9rem;
    padding: 0.6rem;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
  }
  .detected img {
    width: 8rem;
    height: 4.5rem;
    object-fit: cover;
    border-radius: 3px;
    flex: none;
  }
  .detected .text {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
  }
  .radio {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .save-to {
    display: flex;
    gap: 0.7rem;
  }
  .grow {
    flex: 1;
  }
  .rights {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    padding: 0.7rem;
    border: 1px solid var(--warn);
    border-radius: var(--radius);
    font-size: 0.8rem;
    line-height: 1.45;
  }
  .rights :global(svg) {
    color: var(--warn);
    flex: none;
  }
  .tools {
    margin-right: auto;
  }
  .link {
    border: none;
    padding: 0;
    text-transform: none;
    letter-spacing: 0;
    color: var(--accent);
    text-decoration: underline;
    font-size: inherit;
    font-weight: 500;
  }
</style>
