<script lang="ts">
  // A centred window over the app, used by the settings windows. The content goes between
  // the tags:  <Modal title="Project settings" onclose={...}> ...content... </Modal>
  // `children` and `footer` are "snippets": chunks of markup passed in by the parent.
  import { X } from "@lucide/svelte";
  import type { Snippet } from "svelte";
  import { backdrop, rise } from "../lib/motion";
  import { ui } from "../lib/store.svelte";

  interface Props {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
    wide?: boolean;
    /** Use the full window height (Preferences, with its own navigation). */
    tall?: boolean;
    /** No padding around the body (the content lays itself out). */
    bare?: boolean;
  }
  let { title, onclose, children, footer, wide = false, tall = false, bare = false }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    // A question dialog on top gets Escape first.
    if (e.key === "Escape" && !e.defaultPrevented && !ui.question) {
      e.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- `|global`: play when the window is opened or closed, which happens in the parent. -->
<div class="backdrop" transition:backdrop|global>
  <div class="modal" class:wide class:tall transition:rise|global role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <h3 class="display">{title}</h3>
      <button class="icon" onclick={onclose} aria-label="Close"><X size={18} /></button>
    </header>
    <div class="body" class:bare>
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
    background: rgb(0 0 0 / 0.5);
    display: grid;
    place-items: center;
    z-index: 50;
  }
  .modal {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--rule);
    border-radius: 8px;
    box-shadow: var(--shadow);
    width: min(36rem, 94vw);
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal.wide {
    width: min(62rem, 94vw);
  }
  .modal.tall {
    height: min(46rem, 92vh);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.7rem 1rem 0.7rem 1.2rem;
    background: var(--header-bg);
    color: var(--header-text);
  }
  h3 {
    font-size: 1.6rem;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.8rem 1.2rem;
    border-top: 1px solid var(--rule);
  }
  .body {
    padding: 1.2rem;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    flex: 1;
    min-height: 0;
  }
  .body.bare {
    padding: 0;
  }
</style>
