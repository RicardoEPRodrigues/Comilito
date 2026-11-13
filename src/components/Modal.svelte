<script lang="ts">
  // A centred panel over the app, used by the editors. The content goes between the tags:
  //   <Modal title="Edit scene" onclose={...}> ...content... </Modal>
  // `children` and `footer` are "snippets": chunks of markup passed in by the parent.
  import type { Snippet } from "svelte";
  import { ui } from "../lib/store.svelte";

  interface Props {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
    wide?: boolean;
  }
  let { title, onclose, children, footer, wide = false }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    // A question dialog on top gets Escape first.
    if (e.key === "Escape" && !e.defaultPrevented && !ui.question) onclose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="backdrop">
  <div class="modal" class:wide role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h3>{title}</h3>
      <button class="icon" onclick={onclose} aria-label="Close">✕</button>
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.4);
    display: grid;
    place-items: center;
    z-index: 50;
  }
  .modal {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    width: min(34rem, 94vw);
    max-height: 90vh;
    display: flex;
    flex-direction: column;
  }
  .modal.wide {
    width: min(52rem, 94vw);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.8rem 1rem;
  }
  header {
    border-bottom: 1px solid var(--border);
  }
  footer {
    border-top: 1px solid var(--border);
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .body {
    padding: 1rem;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
</style>
