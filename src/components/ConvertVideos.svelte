<script lang="ts">
  // "Convert videos?" (decision 0035, design screen 5). Shown when videos that may not play
  // were added, dropped or found in the project's video folder, or from a video's Convert
  // button. Lists them with a tick each; once started, each row shows its progress. Closing
  // the dialog leaves the conversions running in the background.
  import { Clapperboard, FolderSearch, RefreshCw } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { formatDuration, refreshTools, run, ui } from "../lib/store.svelte";
  import { videoById } from "../lib/video";
  import Modal from "./Modal.svelte";

  const offer = $derived(ui.convertOffer!);
  // Ticked videos; all of them to start with.
  // svelte-ignore state_referenced_locally
  let ticked = $state<string[]>([...ui.convertOffer!.ids]);
  let keepOriginals = $state(false);
  // Opened on conversions already under way (the header's "Converting…"): show their progress.
  let started = $state(
    // svelte-ignore state_referenced_locally
    ui.convertOffer!.ids.every((id) => ui.conversions.some((j) => j.id === id && (j.status === "queued" || j.status === "running"))),
  );
  let error = $state("");

  const videos = $derived(offer.ids.map((id) => videoById(id)).filter((v) => v !== undefined));
  const jobOf = (id: string) => ui.conversions.find((j) => j.id === id);

  function describe(id: string): string {
    const v = videoById(id);
    const i = v?.info;
    if (!v || !i) return "";
    return `${v.file.path.split(".").pop()?.toUpperCase()} · ${i.codec.toUpperCase()} · ${i.width}×${i.height} · ${formatDuration(i.durationMs)}`;
  }

  function status(id: string): string {
    const job = jobOf(id);
    if (!started || !job) return ticked.includes(id) ? "" : "left as it is";
    switch (job.status) {
      case "queued":
        return "waiting";
      case "running":
        return `converting${job.encoder ? " on the GPU" : ""} · ${Math.round(job.progress * 100)}%`;
      case "done":
        return "converted";
      case "cancelled":
        return "cancelled";
      default:
        return job.message ?? "failed";
    }
  }

  async function convert() {
    error = "";
    try {
      await api.convertVideos(ticked, keepOriginals);
      started = true;
    } catch (e) {
      // The tools (ffmpeg) are installed from Preferences or the URL dialog.
      error = String(e) === "TOOLS_MISSING" ? "Converting needs ffmpeg. Install the download tools in Preferences › Projects & storage." : String(e);
      refreshTools();
    }
  }

  function close() {
    ui.convertOffer = null;
  }

  const title = $derived(offer.ids.length === 1 ? "Convert this video?" : `Convert ${offer.ids.length} videos?`);
</script>

<Modal {title} onclose={close}>
  <div class="body">
    {#if offer.origin !== "library"}
      <span class="from muted">
        <FolderSearch size={13} />
        {offer.origin === "folder" ? "Found in the project's video folder" : "Just added"}
      </span>
    {/if}
    <p>
      {offer.ids.length === 1 ? "This video" : "These videos"} may not play on this computer. Converting makes a WebM copy
      that plays everywhere. It runs in the background; you can keep working.
    </p>
    <ul class="list">
      {#each videos as v (v.id)}
        {@const job = jobOf(v.id)}
        <li>
          <div class="top">
            <input
              type="checkbox"
              checked={ticked.includes(v.id)}
              disabled={started}
              onchange={(e) => (ticked = e.currentTarget.checked ? [...ticked, v.id] : ticked.filter((x) => x !== v.id))}
              aria-label={`Convert ${v.name}`}
            />
            <Clapperboard size={15} />
            <div class="text">
              <strong>{v.name}</strong>
              <span class="muted small">{describe(v.id)}</span>
            </div>
            <span class="status" class:active={job?.status === "running"} class:failed={job?.status === "failed"}>{status(v.id)}</span>
            {#if started && job && (job.status === "queued" || job.status === "running")}
              <button class="ghost small" onclick={() => run(api.cancelConversion(v.id))}>Cancel</button>
            {/if}
          </div>
          {#if job?.status === "running"}
            <div class="bar"><i style:width="{job.progress * 100}%"></i></div>
          {/if}
        </li>
      {/each}
    </ul>
    {#if !started}
      <fieldset>
        <span class="caps muted">The originals</span>
        <label><input type="radio" name="originals" checked={!keepOriginals} onchange={() => (keepOriginals = false)} />Move them to the trash once converted (the project keeps only the WebM)</label>
        <label><input type="radio" name="originals" checked={keepOriginals} onchange={() => (keepOriginals = true)} />Keep them in the project as well</label>
      </fieldset>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
  {#snippet footer()}
    {#if started}
      <span class="muted small grow">Closing this leaves the conversions running.</span>
      <button class="primary" onclick={close}>Close</button>
    {:else}
      <span class="muted small grow">Not now: they stay in the Library, marked “may not play”, with a Convert button.</span>
      <button class="ghost" onclick={close}>Not now</button>
      <button class="primary" onclick={convert} disabled={ticked.length === 0}>
        <RefreshCw size={14} />Convert {ticked.length}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    max-width: 38rem;
  }
  p {
    margin: 0;
    line-height: 1.45;
  }
  .from {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    max-height: 18rem;
    overflow: auto;
  }
  li {
    padding: 0.6rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  li + li {
    border-top: 1px solid var(--rule);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .top :global(svg) {
    color: var(--muted);
    flex: none;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .small {
    font-size: 0.75rem;
  }
  .status {
    font-size: 0.75rem;
    color: var(--muted);
    white-space: nowrap;
  }
  .status.active {
    color: var(--accent);
  }
  .status.failed {
    color: var(--danger);
    white-space: normal;
    max-width: 14rem;
  }
  .bar {
    height: 4px;
    background: var(--track);
    border-radius: 2px;
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  fieldset {
    border: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  fieldset label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .error {
    color: var(--danger);
  }
  .grow {
    flex: 1;
  }
</style>
