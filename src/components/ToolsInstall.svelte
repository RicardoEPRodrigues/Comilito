<script lang="ts">
  // Installs yt-dlp and ffmpeg (decision 0019): the app downloads them itself, checks them
  // against their published checksums, and keeps them in its data folder. Shown wherever a
  // feature needs them (downloads from video sites, cropping) and in Preferences.
  import { Download, LoaderCircle } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { formatBytes, notify, refreshTools, ui } from "../lib/store.svelte";

  let { reason = "", onready }: { reason?: string; onready?: () => void } = $props();
  let installing = $state(false);

  onMount(() => {
    refreshTools();
  });

  async function install() {
    installing = true;
    ui.toolsProgress = null;
    try {
      ui.tools = await api.installTools();
      onready?.();
    } catch (err) {
      notify(String(err));
    } finally {
      installing = false;
      ui.toolsProgress = null;
    }
  }

  const progress = $derived(ui.toolsProgress);
</script>

<div class="tools">
  {#if reason}<p>{reason}</p>{/if}
  <p class="muted small">
    Comilito downloads <strong>yt-dlp</strong> (fetches audio from video sites) and <strong>ffmpeg</strong>
    (converts and cuts audio) from their official GitHub releases, checks them against the published
    checksums and keeps them in its own folder. They take about 400 MB.
  </p>
  {#if installing}
    <div class="progress">
      <LoaderCircle size={14} class="spin" />
      <span>
        {#if progress?.total}Downloading {progress.step} · {formatBytes(progress.done)} of {formatBytes(progress.total)}{:else if progress}{progress.step}…{:else}Starting…{/if}
      </span>
    </div>
    {#if progress?.total}
      <div class="track"><div class="fill" style:width="{(progress.done / progress.total) * 100}%"></div></div>
    {/if}
  {:else}
    <button class="primary" onclick={install}><Download size={14} />Download the tools</button>
  {/if}
</div>

<style>
  .tools {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.9rem;
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    background: var(--surface);
  }
  p {
    margin: 0;
    line-height: 1.45;
  }
  .small {
    font-size: 0.8rem;
  }
  button {
    align-self: flex-start;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .track {
    height: 5px;
    background: var(--track);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
