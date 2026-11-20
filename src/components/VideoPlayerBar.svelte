<script lang="ts">
  // Play/pause, position, seek bar and length for the Library's video preview (VideoPage).
  // It replaces WebKit's built-in `controls`: their buttons are drawn from icons inside WebKit,
  // and some versions (the Flatpak runtime's 2.54) draw them empty. Our own bar looks the same
  // everywhere and follows the theme. Videos are always muted (decision 0035), so there's no
  // volume, and the video window is where videos go full screen.
  import { Pause, Play } from "@lucide/svelte";
  import { formatDuration } from "../lib/store.svelte";

  interface Props {
    player: HTMLVideoElement | undefined;
    /** The video's length; from the file's info, so the bar is right before the video loads. */
    lengthMs: number;
    /** While trimming: the part being kept, shaded on the bar. */
    kept?: { start: number; end: number } | null;
  }
  let { player, lengthMs, kept = null }: Props = $props();

  let playing = $state(false);
  let positionMs = $state(0);
  let track = $state<HTMLDivElement>();
  let dragging = false;

  const fraction = (ms: number) => (lengthMs > 0 ? Math.min(1, Math.max(0, ms / lengthMs)) : 0);
  const pct = (ms: number) => `${fraction(ms) * 100}%`;

  // Follow the element. `timeupdate` comes only a few times a second, so while playing the
  // position is also read on every frame, for a smooth bar.
  $effect(() => {
    const el = player;
    if (!el) return;
    let frame = 0;
    const read = () => {
      if (!dragging) positionMs = el.currentTime * 1000;
    };
    const tick = () => {
      read();
      frame = requestAnimationFrame(tick);
    };
    const onPlay = () => {
      playing = true;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(tick);
    };
    const onPause = () => {
      playing = false;
      cancelAnimationFrame(frame);
      read();
    };
    playing = !el.paused;
    read();
    if (playing) onPlay();
    el.addEventListener("play", onPlay);
    el.addEventListener("pause", onPause);
    el.addEventListener("ended", onPause);
    el.addEventListener("seeked", read);
    el.addEventListener("timeupdate", read);
    // Svelte runs this when the element changes or the bar goes away: undo everything above.
    return () => {
      cancelAnimationFrame(frame);
      el.removeEventListener("play", onPlay);
      el.removeEventListener("pause", onPause);
      el.removeEventListener("ended", onPause);
      el.removeEventListener("seeked", read);
      el.removeEventListener("timeupdate", read);
    };
  });

  export function toggle() {
    if (!player) return;
    if (player.paused) player.play().catch(() => {});
    else player.pause();
  }

  function seekTo(ms: number) {
    positionMs = Math.min(lengthMs, Math.max(0, ms));
    if (player) player.currentTime = positionMs / 1000;
  }

  /** The time under the pointer, from its place on the track. */
  function timeAt(e: PointerEvent): number {
    const box = track!.getBoundingClientRect();
    return ((e.clientX - box.left) / box.width) * lengthMs;
  }

  function down(e: PointerEvent) {
    if (!lengthMs || e.button !== 0) return;
    // Keeps the drag going when the pointer leaves the bar.
    track!.setPointerCapture(e.pointerId);
    dragging = true;
    seekTo(timeAt(e));
  }

  function move(e: PointerEvent) {
    if (dragging) seekTo(timeAt(e));
  }

  function up() {
    dragging = false;
  }

  /** Arrow keys step 5 s (Home/End: the ends). Handled here only, so they don't also reach
   *  the app's shortcuts. */
  function key(e: KeyboardEvent) {
    const steps: Record<string, number> = { ArrowLeft: -5000, ArrowRight: 5000 };
    if (e.key in steps) seekTo(positionMs + steps[e.key]);
    else if (e.key === "Home") seekTo(0);
    else if (e.key === "End") seekTo(lengthMs);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<div class="bar">
  <button class="play" onclick={toggle} disabled={!player} title={playing ? "Pause" : "Play"}>
    {#if playing}<Pause size={15} />{:else}<Play size={15} />{/if}
  </button>
  <span class="time">{formatDuration(positionMs)}</span>
  <div
    class="track"
    bind:this={track}
    role="slider"
    tabindex="0"
    aria-label="Position"
    aria-valuemin={0}
    aria-valuemax={Math.round(lengthMs)}
    aria-valuenow={Math.round(positionMs)}
    aria-valuetext={formatDuration(positionMs)}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onkeydown={key}
  >
    <div class="rail">
      {#if kept && kept.end > kept.start}
        <div class="kept" style:left={pct(kept.start)} style:width={`${(fraction(kept.end) - fraction(kept.start)) * 100}%`}></div>
      {/if}
      <div class="played" style:width={pct(positionMs)}></div>
    </div>
    <div class="knob" style:left={pct(positionMs)}></div>
  </div>
  <span class="time">{formatDuration(lengthMs)}</span>
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    flex: none;
  }
  /* The sound editor's round preview button (SoundEditor.svelte). */
  .play {
    width: 2.4rem;
    height: 2.4rem;
    flex: none;
    border-radius: 50%;
    justify-content: center;
    padding: 0;
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .time {
    flex: none;
    min-width: 2.8rem;
    font-size: 0.8rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .time:last-child {
    text-align: right;
  }
  /* Taller than the rail it draws, so it's easy to grab. */
  .track {
    position: relative;
    flex: 1;
    height: 1.4rem;
    cursor: pointer;
    touch-action: none;
  }
  .track:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-radius: var(--radius);
  }
  .rail {
    position: absolute;
    left: 0;
    right: 0;
    top: 50%;
    height: 0.3rem;
    transform: translateY(-50%);
    background: var(--rule);
    border-radius: 999px;
    overflow: hidden;
  }
  .played {
    position: absolute;
    inset: 0 auto 0 0;
    background: var(--accent);
  }
  .kept {
    position: absolute;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .knob {
    position: absolute;
    top: 50%;
    width: 0.85rem;
    height: 0.85rem;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--surface);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }
</style>
