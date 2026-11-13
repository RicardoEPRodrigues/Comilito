<script lang="ts">
  // Bottom bar: play/pause and stop, and the master and category volumes.
  import { Pause, Play, Square } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { run, ui } from "../lib/store.svelte";
  import { CATEGORIES } from "../lib/types";
  import VolumeSlider from "./VolumeSlider.svelte";

  const project = $derived(ui.snapshot!.project!);
  const transport = $derived(ui.snapshot!.playback.transport);
</script>

<footer>
  <button class="play" onclick={() => run(api.togglePlay())} title="Play / pause (Space)" aria-label="Play or pause">
    {#if transport === "playing"}<Pause size={20} />{:else}<Play size={20} />{/if}
  </button>
  <button class="stop" onclick={() => run(api.stopAll())} title="Stop all (Esc)" aria-label="Stop all"><Square size={13} /></button>
  <span class="divider"></span>
  <VolumeSlider label="Master" value={project.masterVolume} onchange={(v) => run(api.setMasterVolume(v))} />
  {#each CATEGORIES as c (c.id)}
    <VolumeSlider label={c.label} value={project.categories[c.id].volume} onchange={(v) => run(api.setCategoryVolume(c.id, v))} />
  {/each}
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
