<script lang="ts">
  // Bottom bar: play/pause and stop, the project's master volume, and the category volumes
  // of the top-most enabled scene (decision 0028). While a sound is previewed, a chip names it
  // and stops it, wherever it was started from.
  import { Film, Headphones, Pause, Play, Square } from "@lucide/svelte";
  import { videoById, videoEnabled } from "../lib/video";
  import { formatDuration } from "../lib/store.svelte";
  import { api } from "../lib/api";
  import { run, sceneById, soundById, ui } from "../lib/store.svelte";
  import { CATEGORIES } from "../lib/types";
  import VolumeSlider from "./VolumeSlider.svelte";

  const project = $derived(ui.snapshot!.project!);
  const mix = $derived(ui.snapshot!.mix);
  const transport = $derived(ui.snapshot!.playback.transport);
  const topScene = $derived(mix.volumesScene ? sceneById(mix.volumesScene) : undefined);
  const previewing = $derived(ui.snapshot!.playback.preview ? soundById(ui.snapshot!.playback.preview) : undefined);

  // The video window's report (about once a second): the video, or the video effect, showing.
  const status = $derived(ui.videoStatus);
  const nowVideo = $derived(status?.effect ? videoById(status.effect) : status?.videoId ? videoById(status.videoId) : undefined);
  const nowMeta = $derived.by(() => {
    if (!status) return "";
    if (status.effect) return "Video effect";
    const scene = status.sceneId ? ui.snapshot?.project?.scenes.find((s) => s.id === status.sceneId)?.name : null;
    const parts = ["Video", scene, formatDuration(status.positionMs)];
    if (status.switchInMs !== null) parts.push(`switches in ${formatDuration(status.switchInMs)}`);
    return parts.filter(Boolean).join(" · ");
  });
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
        <VolumeSlider
          label={c.label}
          value={mix.volumes[c.id]}
          glideMs={ui.snapshot!.levelsFadeMs}
          onchange={(v) => run(api.setCategoryVolume(c.id, v))}
        />
      {/each}
    </div>
  </div>
  <span class="spacer"></span>
  {#if videoEnabled() && ui.snapshot!.videoWindowOpen && nowVideo}
    <!-- What the video window shows (decision 0035); clicking brings the window forward. -->
    <button class="now-video" onclick={() => run(api.showVideoWindow())} title="Bring the video window forward">
      <Film size={14} />
      <span class="text">
        <span class="name">{nowVideo.name}</span>
        <span class="meta">{nowMeta}</span>
      </span>
    </button>
  {/if}
  {#if previewing}
    <button class="preview" onclick={() => run(api.stopPreview())} title="Stop the preview">
      <Headphones size={14} /><span class="name">{previewing.name}</span><Square size={11} />
    </button>
  {/if}
  <span class="hint">
    {transport === "stopped" ? "Stopped" : transport === "paused" ? "Paused" : "Playing"} · Space pause · Esc stop
  </span>
</footer>

<style>
  .now-video {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    max-width: 18rem;
    padding: 0.3rem 0.6rem;
    color: inherit;
    border-color: color-mix(in srgb, currentColor 25%, transparent);
    text-transform: none;
    letter-spacing: 0;
  }
  .now-video .text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
  }
  .now-video .name {
    font-size: 0.78rem;
    font-weight: 600;
    max-width: 15rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .now-video .meta {
    font-size: 0.68rem;
    opacity: 0.7;
    white-space: nowrap;
  }
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
  .preview {
    gap: 0.45rem;
    max-width: 16rem;
    border-color: var(--mixer-text);
    color: var(--mixer-text);
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.8rem;
    padding: 0.35rem 0.6rem;
  }
  .preview .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    font-size: 0.65rem;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    opacity: 0.55;
  }
</style>
