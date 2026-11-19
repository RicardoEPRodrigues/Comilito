<script lang="ts">
  // The mini player (decision 0049; designs I–I3 in pencil/comilito.pen): the main window,
  // small and on top of other windows, for running a session next to something else (a
  // virtual tabletop, notes). Also the layout of a window too narrow for the full one, but
  // without going on top or the buttons below that only make sense in the mini player.
  //
  //   title row: project, Video, pin (on top), back to the full window
  //   Tapestry | Effects tabs
  //   Tapestry tab: the stack, then the Scenes / Presets drawer
  //   controls: play, stop, master; the top scene's Music / Ambience / Effects
  import { Maximize2, MonitorPlay, Pause, Pin, PinOff, Play, Square } from "@lucide/svelte";
  import { api } from "../lib/api";
  import { remember, run, sceneById, setMiniOnTop, setMiniPlayer, ui } from "../lib/store.svelte";
  import { CATEGORIES } from "../lib/types";
  import { videoEnabled } from "../lib/video";
  import EffectsPad from "./EffectsPad.svelte";
  import MiniScenes from "./MiniScenes.svelte";
  import MiniTapestry from "./MiniTapestry.svelte";
  import VolumeSlider from "./VolumeSlider.svelte";

  const snapshot = $derived(ui.snapshot!);
  const project = $derived(snapshot.project!);
  const mix = $derived(snapshot.mix);
  const transport = $derived(snapshot.playback.transport);
  // Whose volumes the sliders show: the top enabled scene's (decision 0028), as in the mixer bar.
  const topScene = $derived(mix.volumesScene ? sceneById(mix.volumesScene) : undefined);

  function select(tab: "tapestry" | "effects") {
    ui.miniTab = tab;
    remember("miniTab", tab);
  }
</script>

<div class="mini">
  <header>
    <span class="title display" title={project.name}>{project.name}</span>
    {#if videoEnabled()}
      <button
        class="video-toggle"
        class:on={snapshot.videoWindowOpen}
        title={snapshot.videoWindowOpen ? "Close the video window" : "Open the video window"}
        onclick={() => run(api.toggleVideoWindow())}
      >
        <MonitorPlay size={14} />Video
      </button>
    {/if}
    {#if ui.mini.on}
      <button
        class="icon"
        class:on={ui.mini.onTop}
        title={ui.mini.onTop ? "Stop keeping this window on top" : "Keep this window on top of others"}
        onclick={() => setMiniOnTop(!ui.mini.onTop)}
      >
        {#if ui.mini.onTop}<Pin size={16} />{:else}<PinOff size={16} />{/if}
      </button>
      <button class="icon" title="Back to the full window" onclick={() => setMiniPlayer(false)}><Maximize2 size={16} /></button>
    {/if}
  </header>

  {#if snapshot.changedOnDisk}
    <!-- As in the full window: someone else changed comilito.json, and only the user knows which to keep. -->
    <div class="banner">
      <span>comilito.json was changed by another program.</span>
      <button onclick={() => run(api.reloadProject())}>Reload</button>
      <button onclick={() => run(api.keepMyVersion())}>Keep mine</button>
    </div>
  {/if}

  <div class="tabs" role="tablist">
    <button role="tab" class:active={ui.miniTab === "tapestry"} aria-selected={ui.miniTab === "tapestry"} onclick={() => select("tapestry")}>Tapestry</button>
    <button role="tab" class:active={ui.miniTab === "effects"} aria-selected={ui.miniTab === "effects"} onclick={() => select("effects")}>Effects</button>
  </div>

  <main>
    {#if ui.miniTab === "tapestry"}
      <MiniTapestry />
      <MiniScenes />
    {:else}
      <EffectsPad compact />
    {/if}
  </main>

  <footer>
    <div class="transport">
      <button class="play" onclick={() => run(api.togglePlay())} title="Play / pause (Space)" aria-label="Play or pause">
        {#if transport === "playing"}<Pause size={17} />{:else}<Play size={17} />{/if}
      </button>
      <button class="stop" onclick={() => run(api.stopAll())} title="Stop all (Esc)" aria-label="Stop all"><Square size={12} /></button>
      <span class="divider"></span>
      <VolumeSlider label="Master" value={project.masterVolume} onchange={(v) => run(api.setMasterVolume(v))} />
    </div>
    <div
      class="levels"
      title={topScene
        ? `Volumes of ${topScene.name}, the top scene of the tapestry${topScene.volumes ? "" : " (project volumes until changed)"}`
        : "Project volumes, used by scenes that have none of their own"}
    >
      <span class="owner caps">{topScene ? topScene.name : "Project volumes"}</span>
      <div class="sliders">
        {#each CATEGORIES as c (c.id)}
          <VolumeSlider label={c.label} value={mix.volumes[c.id]} glideMs={snapshot.levelsFadeMs} onchange={(v) => run(api.setCategoryVolume(c.id, v))} />
        {/each}
      </div>
    </div>
  </footer>
</div>

<style>
  .mini {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    height: 2.75rem;
    padding: 0 0.5rem 0 0.9rem;
    background: var(--header-bg);
    color: var(--header-text);
  }
  .title {
    flex: 1;
    min-width: 0;
    font-size: 1.3rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  header .icon {
    color: inherit;
  }
  header .on {
    background: color-mix(in srgb, currentColor 18%, transparent);
  }
  .video-toggle {
    color: inherit;
    border-color: currentColor;
    padding: 0.25rem 0.5rem;
    margin-right: 0.2rem;
    font-size: 0.6rem;
    gap: 0.3rem;
  }
  .banner {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem 0.5rem;
    padding: 0.4rem 0.9rem;
    background: color-mix(in srgb, var(--warn) 18%, var(--bg));
    border-bottom: 1px solid var(--rule);
    font-size: 0.78rem;
  }
  .banner span {
    flex: 1 1 100%;
  }
  .banner button {
    padding: 0.2rem 0.5rem;
  }
  .tabs {
    flex: none;
    display: flex;
    gap: 1.4rem;
    padding: 0.6rem 0.9rem 0;
    border-bottom: 1px solid var(--rule);
  }
  .tabs button {
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    padding: 0 0 0.45rem;
    font-size: 0.68rem;
    color: var(--muted);
  }
  .tabs button:hover {
    background: none !important;
    color: var(--text);
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--text);
  }
  main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  footer {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.6rem 0.9rem 0.75rem;
    background: var(--mixer-bg);
    color: var(--mixer-text);
  }
  .transport {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }
  /* The sliders share the width instead of their fixed size in the mixer bar. */
  footer :global(label) {
    flex: 1;
    min-width: 0;
    width: auto;
  }
  .play {
    flex: none;
    width: 2.4rem;
    height: 2.4rem;
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
    flex: none;
    width: 1.9rem;
    height: 1.9rem;
    border-radius: 50%;
    justify-content: center;
    padding: 0;
    border-color: var(--mixer-text);
    color: var(--mixer-text);
  }
  .divider {
    flex: none;
    width: 1px;
    height: 1.9rem;
    background: color-mix(in srgb, var(--mixer-text) 25%, transparent);
  }
  .levels {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .owner {
    font-size: 0.58rem;
    opacity: 0.55;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sliders {
    display: flex;
    gap: 0.9rem;
  }
</style>
