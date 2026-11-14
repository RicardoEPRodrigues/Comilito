<script lang="ts">
  // The download queue (decision 0019), at the bottom of the Library tree. Downloads run one
  // at a time in the background; finished ones become sounds and can be opened from here.
  import { Check, Download, RotateCcw, TriangleAlert, X } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { openSoundInLibrary, run, ui } from "../lib/store.svelte";

  const jobs = $derived(ui.downloads);
  const active = $derived(jobs.filter((j) => j.status === "running" || j.status === "queued").length);
  const finished = $derived(jobs.length - active);
</script>

{#if jobs.length > 0}
  <section>
    <div class="head">
      <Download size={13} />
      <span class="caps">Downloads</span>
      <span class="spacer"></span>
      {#if finished > 0}<button class="link" onclick={() => run(api.clearDownloads())}>Clear</button>{/if}
      <span class="n">{active ? `${active} active` : "done"}</span>
    </div>
    {#each jobs as job (job.id)}
      <div class="job" class:failed={job.status === "failed"}>
        <div class="row">
          {#if job.status === "done"}<Check size={12} />{:else if job.status === "failed"}<TriangleAlert size={12} />{/if}
          {#if job.status === "done" && job.soundId}
            <button class="name link" onclick={() => openSoundInLibrary(job.soundId!)}>{job.name}</button>
          {:else}
            <span class="name">{job.name}</span>
          {/if}
          {#if job.status === "running" || job.status === "queued"}
            <button class="icon" title="Cancel" onclick={() => run(api.cancelDownload(job.id))}><X size={12} /></button>
          {/if}
        </div>
        <span class="meta">
          {job.site} ·
          {#if job.status === "queued"}{job.message ?? "queued"}{:else if job.status === "running"}{job.message ?? `${Math.round(job.progress * 100)}%`}{:else if job.status === "done"}added to the Library{:else if job.status === "cancelled"}cancelled{:else}{job.message}{/if}
        </span>
        {#if job.status === "running"}
          <div class="track"><div class="fill" style:width="{job.progress * 100}%"></div></div>
        {/if}
      </div>
    {/each}
    {#if jobs.some((j) => j.status === "failed" && j.message?.includes("aren't installed"))}
      <button class="install" onclick={() => (ui.editing = { kind: "addUrl" })}><RotateCcw size={12} />Install the download tools</button>
    {/if}
  </section>
{/if}

<style>
  section {
    border: 1px solid var(--side-muted);
    border-radius: var(--radius);
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    max-height: 14rem;
    overflow: auto;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .caps {
    font-size: 0.6rem;
  }
  .spacer {
    flex: 1;
  }
  .n,
  .meta {
    font-size: 0.7rem;
    color: var(--side-muted);
  }
  .job {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .job.failed .meta {
    color: var(--warn);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .name {
    flex: 1;
    font-size: 0.8rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }
  .link {
    border: none;
    padding: 0;
    text-transform: none;
    letter-spacing: 0;
    color: inherit;
    text-decoration: underline;
    font-size: 0.72rem;
  }
  .name.link {
    font-size: 0.8rem;
    justify-content: flex-start;
  }
  .icon {
    padding: 0.1rem;
    color: var(--side-muted);
  }
  .track {
    height: 4px;
    background: var(--track);
    border-radius: 2px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
  }
  .install {
    font-size: 0.6rem;
    border-color: var(--side-text);
    justify-content: center;
  }
</style>
