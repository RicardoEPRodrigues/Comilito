<script lang="ts">
  // Covers the app while the projects folder is being moved (decision 0023). The move runs
  // on a Rust thread, so the window keeps redrawing; this dialog shows how far it got and
  // keeps the user from opening or changing projects meanwhile.
  import { FolderSync } from "@lucide/svelte";
  import { formatBytes, ui } from "../lib/store.svelte";

  const fraction = $derived(ui.moving && ui.moving.totalBytes > 0 ? ui.moving.doneBytes / ui.moving.totalBytes : 0);
  /** Just the file's name; the full path is too long to read. */
  const current = $derived(ui.moving?.current.split(/[\\/]/).pop() ?? "");
</script>

{#if ui.moving}
  <div class="backdrop">
    <div class="dialog" role="alertdialog" aria-modal="true" aria-label="Moving the projects">
      <div class="heading"><FolderSync size={20} /><h3 class="display">Moving the projects</h3></div>
      <div class="bar" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(fraction * 100)}>
        <span style:width="{fraction * 100}%"></span>
      </div>
      <div class="status">
        <span class="file">{current || "Getting ready…"}</span>
        {#if ui.moving.totalBytes > 0}
          <span class="numbers">{formatBytes(ui.moving.doneBytes)} of {formatBytes(ui.moving.totalBytes)}</span>
        {/if}
      </div>
      <p class="muted">Keep Comilito open until this finishes. Moving to another disk copies every file, which can take a while.</p>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.55);
    display: grid;
    place-items: center;
    z-index: 200;
  }
  .dialog {
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: 8px;
    box-shadow: var(--shadow);
    padding: 1.4rem;
    width: min(30rem, 92vw);
    display: flex;
    flex-direction: column;
    gap: 0.9rem;
  }
  .heading {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--accent);
  }
  h3 {
    color: var(--text);
    font-size: 1.7rem;
  }
  .bar {
    height: 8px;
    background: var(--track);
    border-radius: 4px;
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.15s;
  }
  .status {
    display: flex;
    gap: 1rem;
    font-size: 0.82rem;
  }
  .file {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .numbers {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  p {
    margin: 0;
    font-size: 0.82rem;
  }
</style>
