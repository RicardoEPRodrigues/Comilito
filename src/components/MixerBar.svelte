<script lang="ts">
  // Bottom bar: transport (play/pause, stop) and the master and category volumes.
  import { api } from "../lib/api";
  import { run, ui } from "../lib/store.svelte";
  import { CATEGORIES } from "../lib/types";
  import VolumeSlider from "./VolumeSlider.svelte";

  const project = $derived(ui.snapshot!.project!);
  const transport = $derived(ui.snapshot!.playback.transport);
</script>

<footer>
  <div class="transport">
    <button class="play" onclick={() => run(api.togglePlay())} title="Play / pause (Space)">
      {transport === "playing" ? "⏸ Pause" : "▶ Play"}
    </button>
    <button onclick={() => run(api.stopAll())} title="Stop all (Esc)">⏹ Stop</button>
    <span class="status muted">
      {transport === "stopped" ? "Stopped" : transport === "paused" ? "Paused" : "Playing"}
    </span>
  </div>

  <div class="sliders">
    <VolumeSlider
      label="Master"
      value={project.masterVolume}
      onchange={(v) => run(api.setMasterVolume(v))}
    />
    {#each CATEGORIES as c (c.id)}
      <VolumeSlider
        label={c.label}
        value={project.categories[c.id].volume}
        onchange={(v) => run(api.setCategoryVolume(c.id, v))}
      />
    {/each}
  </div>
</footer>

<style>
  footer {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    padding: 0.5rem 0.8rem;
    border-top: 1px solid var(--border);
    background: var(--panel);
    flex-wrap: wrap;
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .play {
    min-width: 6rem;
  }
  .status {
    width: 4.5rem;
  }
  .sliders {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 1.2rem;
  }
</style>
