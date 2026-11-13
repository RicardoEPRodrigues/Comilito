<script lang="ts">
  // Right panel: buttons for the effects of every enabled scene, top scene first.
  import { api } from "../lib/api";
  import { run, soundById, ui } from "../lib/store.svelte";

  const snapshot = $derived(ui.snapshot!);
  const looping = $derived(new Set(snapshot.playback.loopingEffects));
  const missing = $derived(new Set(snapshot.missing));
</script>

<aside>
  <h2>Effects</h2>
  {#if snapshot.mix.effects.length === 0}
    <p class="muted">Effects of the scenes in the stack appear here.</p>
  {/if}
  <div class="pad">
    {#each snapshot.mix.effects as id (id)}
      {@const sound = soundById(id)}
      {#if sound}
        <button
          class:active={looping.has(id)}
          disabled={missing.has(id)}
          onclick={() => run(api.playEffect(id))}
          title={sound.loop ? "Looping effect: click again to stop" : "Play"}
        >
          {sound.name}
        </button>
      {/if}
    {/each}
  </div>
</aside>

<style>
  aside {
    border-left: 1px solid var(--border);
    background: var(--panel);
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-height: 0;
    overflow: auto;
  }
  p {
    margin: 0;
  }
  .pad {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
    gap: 0.4rem;
  }
  .pad button {
    min-height: 3.2rem;
    padding: 0.4rem;
    overflow-wrap: anywhere;
  }
  .pad button.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, var(--panel-2));
  }
</style>
