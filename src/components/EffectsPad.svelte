<script lang="ts">
  // Right panel: buttons for the effects of every enabled scene, top scene first.
  // Clicking an effect plays it; clicking it again while it plays stops it. Below them, the
  // video effects (decision 0035), which share the column.
  // `compact` is the mini player's Effects tab (decision 0049): its tab names it, and video
  // effects are left to the full window.
  import { fitText } from "../lib/fit";
  import { Film, Zap } from "@lucide/svelte";
  import { videoById, videoEnabled } from "../lib/video";
  import { api } from "../lib/api";
  import { run, sceneById, soundById, ui } from "../lib/store.svelte";

  let { compact = false }: { compact?: boolean } = $props();

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

<aside class:compact>
  <!-- Sound effects take the room the video effects leave, and scroll in it. -->
  <div class="area sounds">
    {#if !compact}<h2 class="display" use:fitText>Effects</h2>{/if}
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
  </div>
  {#if !compact && videoEnabled() && snapshot.mix.videoEffects.length > 0}
    <!-- Video effects (decision 0035): as tall as they need, up to half the column. -->
    <div class="area videos">
      <h2 class="display" use:fitText>Video effects</h2>
      <span class="caps source">Replaces the scene's video until it ends</span>
      <div class="pad">
        {#each snapshot.mix.videoEffects as id (id)}
          {@const video = videoById(id)}
          {#if video}
            {@const on = snapshot.videoEffect === id}
            <button class:active={on} disabled={missing.has(id)} onclick={() => run(api.playVideoEffect(id))} title={on ? "Click to stop it" : "Play it in the video window"}>
              <Film size={13} />
              <span>{video.name}</span>
            </button>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</aside>

<style>
  aside {
    background: var(--fx-bg);
    border-left: 1px solid var(--rule);
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  aside.compact {
    flex: 1;
    background: none;
    border-left: none;
  }
  aside.compact .area {
    padding: 0.8rem 0.75rem;
    gap: 0.5rem;
  }
  .area {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    padding: 1.4rem 1.1rem;
    min-height: 0;
    overflow: auto;
  }
  /* Basis 0: the sound effects get only the room left over, never pushing the videos out. */
  .area.sounds {
    flex: 1 1 0;
  }
  .area.videos {
    flex: 0 0 auto;
    max-height: 50%;
    padding-top: 1rem;
    border-top: 1px solid var(--rule);
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
