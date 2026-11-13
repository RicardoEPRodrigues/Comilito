<script lang="ts">
  // The woodcut skeletal horse, drawn faintly behind a panel's content (decision 0016).
  // The parent must be `position: relative`. Hidden when "Woodcut watermark" is off.
  import horse from "../assets/skeletal-horse.svg";
</script>

<img class="watermark" src={horse} alt="" aria-hidden="true" draggable="false" />

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
  :global(:root.no-watermark) .watermark {
    display: none;
  }
</style>
