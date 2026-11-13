<script lang="ts">
  // The woodcut drawn faintly behind a panel's content (decision 0016): the skeletal horse,
  // or the active theme's own woodcut (decision 0025). The parent must be
  // `position: relative`. Hidden when "Woodcut watermark" is off.
  import horse from "../assets/skeletal-horse.svg";
  import { ui } from "../lib/store.svelte";
</script>

{#if ui.themeWatermark}
  <!-- A theme's woodcut is used only for its shape (a CSS mask) and painted in the theme's
       gold, so it always suits the theme's colours, including after they're edited. -->
  <div class="watermark themed" style:--woodcut="url({ui.themeWatermark})" aria-hidden="true"></div>
{:else}
  <img class="watermark" src={horse} alt="" aria-hidden="true" draggable="false" />
{/if}

<style>
  .watermark {
    position: absolute;
    right: 1rem;
    bottom: 0.5rem;
    width: min(46%, 30rem);
    opacity: 0.35;
    pointer-events: none;
    z-index: 0;
  }
  :global(:root[data-dark="true"]) .watermark {
    opacity: 0.12;
  }
  .themed {
    height: min(80%, 30rem);
    background: var(--gold);
    /* WebKitGTK (Linux) still needs the -webkit- prefix for masks. */
    -webkit-mask: var(--woodcut) right bottom / contain no-repeat;
    mask: var(--woodcut) right bottom / contain no-repeat;
  }
  /* Painted in gold rather than drawn light-on-dark like the horse, so it needs more opacity
     to show on a dark page. */
  :global(:root[data-dark="true"]) .themed {
    opacity: 0.2;
  }
  :global(:root.no-watermark) .watermark {
    display: none;
  }
</style>
