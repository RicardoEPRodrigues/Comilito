<script lang="ts">
  // "Add audio from a URL" (decision 0019): paste a video page or a direct link to an audio
  // file, check what it is, choose the part, name, category and folder, then download. The
  // download runs in the background (see Downloads in the Library). Nothing is streamed.
  import { Globe, LoaderCircle, Link, Scale, Download as DownloadIcon } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { formatBytes, formatDuration, refreshTools, run, ui } from "../lib/store.svelte";
  import { CATEGORIES, type Category, type Probe } from "../lib/types";
  import Modal from "./Modal.svelte";
  import ToolsInstall from "./ToolsInstall.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let url = $state("");
  let probing = $state(false);
  let probe = $state<Probe | null>(null);
  let error = $state("");
  let needTools = $state(false);
  let partial = $state(false);
  let from = $state("0:00");
  let to = $state("10:00");
  let name = $state("");
  // svelte-ignore state_referenced_locally
  let category = $state<Category>(ui.library.category ?? "ambience");
  // svelte-ignore state_referenced_locally
  let folder = $state(ui.library.category && ui.library.folder ? ui.library.folder : "");
  let updateAvailable = $state<boolean | null>(null);

  onMount(() => {
    refreshTools();
    api.toolsUpdateAvailable().then((v) => (updateAvailable = v)).catch(() => {});
  });

  async function check() {
    if (!url.trim()) return;
    probing = true;
    error = "";
    probe = null;
    needTools = false;
    try {
      probe = await api.probeUrl(url.trim());
      name = probe.title;
      if (probe.durationMs && probe.durationMs > 20 * 60 * 1000) {
        // Long ambience videos: suggest a 10-minute part.
        partial = true;
        from = "0:00";
        to = "10:00";
      } else {
        partial = false;
        to = probe.durationMs ? formatDuration(probe.durationMs) : "10:00";
      }
    } catch (e) {
      const message = String(e);
      if (message.includes("TOOLS_MISSING")) needTools = true;
      else error = message;
    } finally {
      probing = false;
    }
  }

  function parseTime(text: string): number | null {
    const parts = text.trim().split(":").map(Number);
    if (parts.length === 0 || parts.some((p) => Number.isNaN(p))) return null;
    return Math.round(parts.reduce((acc, p) => acc * 60 + p, 0) * 1000);
  }

  const range = $derived.by(() => {
    if (!partial) return null;
    const startMs = parseTime(from);
    const endMs = parseTime(to);
    if (startMs === null || endMs === null || endMs <= startMs) return null;
    return { startMs, endMs };
  });
  const estimate = $derived.by(() => {
    if (!probe?.size) return null;
    if (!range || !probe.durationMs) return probe.size;
    return Math.round((probe.size * (range.endMs - range.startMs)) / probe.durationMs);
  });
  const folders = $derived(ui.snapshot?.folders[category] ?? []);
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
        range,
      }),
    );
    if (id) onclose();
  }
</script>

<Modal title="Add audio from a URL" {onclose}>
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

    <div class="field">
      <span class="caps">What to download</span>
      <label class="radio"><input type="radio" bind:group={partial} value={false} />Everything {#if probe.size}<span class="muted small">≈ {formatBytes(probe.size)}</span>{/if}</label>
      <label class="radio">
        <input type="radio" bind:group={partial} value={true} />Only from
        <input class="time" bind:value={from} onfocus={() => (partial = true)} /> to
        <input class="time" bind:value={to} onfocus={() => (partial = true)} />
        {#if partial && estimate}<span class="muted small">≈ {formatBytes(estimate)} · long ambience loops rarely need hours</span>{/if}
      </label>
      {#if partial && !range}<span class="error small">The end must be after the start (times like 5:00 or 1:02:30).</span>{/if}
    </div>

    <div class="save-to">
      <label class="field grow"><span class="caps">Name</span><input bind:value={name} /></label>
      <label class="field">
        <span class="caps">Category</span>
        <select bind:value={category} onchange={() => (folder = "")}>
          {#each CATEGORIES as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
        </select>
      </label>
      <label class="field">
        <span class="caps">Folder</span>
        <select bind:value={folder}>
          <option value="">(category folder)</option>
          {#each folders as f (f)}<option value={f}>{f}</option>{/each}
        </select>
      </label>
    </div>

    {#if probe.kind === "page" && ui.tools && !ui.tools.ffmpeg}
      <ToolsInstall reason="This download needs yt-dlp and ffmpeg." />
    {/if}

    <div class="rights">
      <Scale size={15} />
      <span>
        Only download audio you have the right to use. Many sites' terms don't allow downloading;
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
  .time {
    width: 5.5rem;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
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
