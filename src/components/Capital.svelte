<script lang="ts">
  // A decorated capital, as at the start of a chapter in an illuminated manuscript: the first
  // letter of a name in the display font, on a coloured square. `live` uses the accent colour
  // (what's playing); otherwise gold. Hidden when "Decorated capitals" is off.
  let { name, size = 46, live = false }: { name: string; size?: number; live?: boolean } = $props();
  const letter = $derived((name.trim()[0] ?? "?").toUpperCase());
</script>

<span class="capital display" class:live style:width="{size}px" style:height="{size}px" style:font-size="{size * 0.72}px" aria-hidden="true">
  {letter}
</span>

<style>
  .capital {
    flex: none;
    display: inline-grid;
    place-items: center;
    background: var(--gold);
    color: var(--text);
    border-radius: 2px;
    line-height: 1;
    padding-top: 0.06em;
  }
  .capital.live {
    background: var(--accent);
    color: var(--accent-text);
  }
  :global(:root.no-capitals) .capital {
    display: none;
  }
</style>
