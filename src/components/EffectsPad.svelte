<script lang="ts">
  // Right panel: buttons for the effects of every enabled scene, top scene first.
  // Clicking an effect plays it; clicking it again while it plays stops it.
  import { fitText } from "../lib/fit";
  import { Zap } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { run, sceneById, soundById, ui } from "../lib/store.svelte";

  const snapshot = $derived(ui.snapshot!);
  const playing = $derived(new Set(snapshot.playback.playingEffects));
  const missing = $derived(new Set(snapshot.missing));
  /** Names of the enabled scenes the effects come from, top first. */
  const sources = $derived(
    [...snapshot.project!.stack]
      .reverse()
      .filter((e) => e.enabled)
      .map((e) => sceneById(e.scene))
      .filter((s) => s && s.effects.length > 0)
      .map((s) => s!.name),
  );
</script>

<aside>
  <h2 class="display" use:fitText>Effects</h2>
  {#if sources.length}<span class="caps source">From {[...new Set(sources)].join(" · ")}</span>{/if}
  {#if snapshot.mix.effects.length === 0}
    <p class="muted">The effects of the scenes in the tapestry appear here.</p>
  {/if}
  <div class="pad">
    {#each snapshot.mix.effects as id (id)}
      {@const sound = soundById(id)}
      {#if sound}
        <button
          class:active={playing.has(id)}
          disabled={missing.has(id)}
          onclick={() => run(api.toggleEffect(id))}
          title={playing.has(id) ? "Click to stop" : "Play"}
        >
          <Zap size={13} />
          <span>{sound.name}</span>
        </button>
      {/if}
    {/each}
  </div>
</aside>

<style>
  aside {
    background: var(--fx-bg);
    border-left: 1px solid var(--rule);
    padding: 1.4rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    min-height: 0;
    min-width: 0;
    overflow: auto;
  }
  h2 {
    font-size: 1.9rem;
  }
  .source {
    color: var(--muted);
    font-size: 0.6rem;
    line-height: 1.6;
  }
  p {
    margin: 0;
    font-size: 0.85rem;
    line-height: 1.45;
  }
  .pad {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(6.8rem, 1fr));
    gap: 0.55rem;
  }
  .pad button {
    min-height: 4.4rem;
    padding: 0.6rem;
    flex-direction: column;
    align-items: flex-start;
    justify-content: space-between;
    background: var(--surface);
    border-color: var(--rule);
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.85rem;
    font-weight: 600;
    white-space: normal;
    text-align: left;
    color: var(--text);
  }
  .pad button :global(svg) {
    color: var(--accent);
  }
  .pad button.active {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .pad button.active :global(svg) {
    color: var(--accent-text);
  }
</style>
