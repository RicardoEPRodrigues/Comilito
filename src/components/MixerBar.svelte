<script lang="ts">
  // Bottom bar: play/pause and stop, the project's master volume, and the category volumes
  // of the top-most enabled scene (decision 0028).
  import { Pause, Play, Square } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { run, sceneById, ui } from "../lib/store.svelte";
  import { CATEGORIES } from "../lib/types";
  import VolumeSlider from "./VolumeSlider.svelte";

  const project = $derived(ui.snapshot!.project!);
  const mix = $derived(ui.snapshot!.mix);
  const transport = $derived(ui.snapshot!.playback.transport);
  const topScene = $derived(mix.volumesScene ? sceneById(mix.volumesScene) : undefined);
</script>

<footer>
  <button class="play" onclick={() => run(api.togglePlay())} title="Play / pause (Space)" aria-label="Play or pause">
    {#if transport === "playing"}<Pause size={20} />{:else}<Play size={20} />{/if}
  </button>
  <button class="stop" onclick={() => run(api.stopAll())} title="Stop all (Esc)" aria-label="Stop all"><Square size={13} /></button>
  <span class="divider"></span>
  <VolumeSlider label="Master" value={project.masterVolume} onchange={(v) => run(api.setMasterVolume(v))} />
  <span class="divider"></span>
  <div
    class="scene-levels"
    title={topScene
      ? `Volumes of ${topScene.name}, the top scene of the tapestry${topScene.volumes ? "" : " (project volumes until changed)"}`
      : "Project volumes, used by scenes that have none of their own"}
  >
    <span class="owner caps">{topScene ? topScene.name : "Project volumes"}</span>
    <div class="sliders">
      {#each CATEGORIES as c (c.id)}
        <VolumeSlider label={c.label} value={mix.volumes[c.id]} onchange={(v) => run(api.setCategoryVolume(c.id, v))} />
      {/each}
    </div>
  </div>
  <span class="spacer"></span>
  <span class="hint">
    {transport === "stopped" ? "Stopped" : transport === "paused" ? "Paused" : "Playing"} · Space pause · Esc stop
  </span>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    gap: 1.6rem;
    padding: 0 1.4rem;
    height: 4.6rem;
    flex: none;
    background: var(--mixer-bg);
    color: var(--mixer-text);
  }
  .play {
    width: 2.9rem;
    height: 2.9rem;
    border-radius: 50%;
    justify-content: center;
    padding: 0;
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .play:hover {
    background: color-mix(in srgb, var(--accent) 85%, black) !important;
  }
  .stop {
    width: 2.2rem;
    height: 2.2rem;
    border-radius: 50%;
    justify-content: center;
    padding: 0;
    border-color: var(--mixer-text);
    color: var(--mixer-text);
  }
  .divider {
    width: 1px;
    height: 2.2rem;
    background: color-mix(in srgb, var(--mixer-text) 25%, transparent);
  }
  .scene-levels {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    min-width: 0;
  }
  .owner {
    font-size: 0.6rem;
    opacity: 0.55;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 30rem;
  }
  .sliders {
    display: flex;
    gap: 1.6rem;
  }
  .spacer {
    flex: 1;
  }
  .hint {
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    opacity: 0.55;
  }
</style>
