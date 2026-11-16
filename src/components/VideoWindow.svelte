<script lang="ts">
  // The video window (decision 0035, design screen 3). Everything in this window is this
  // component: main.ts mounts it instead of App when the window's label is "video".
  //
  // Rust says *what* should show: `mix.video` (the top scene's playlist, settings filled in),
  // the transport (Space pauses, Esc stops) and the playing video effect. This decides *how*:
  // which video of the playlist, when to switch, crossfades, and remembering where each video
  // stopped (kept in Rust so it survives closing this window).
  //
  // Two <video> layers take turns, so a new video fades in over the old one. A third layer on
  // top plays video effects. Every <video> is muted: a video's sound is never played.
  import { Film, Grip, Maximize, Minimize, Monitor, X } from "@lucide/svelte";
  import { listen, emit } from "@tauri-apps/api/event";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onDestroy, onMount, untrack } from "svelte";
  import { api } from "../lib/api";
  import { formatDuration, ui } from "../lib/store.svelte";
  import type { Preferences, Screen, Snapshot, VideoTarget } from "../lib/types";
  import { videoById } from "../lib/video";
  import { loadVideo, type Playback } from "../lib/videoStream";

  const win = getCurrentWebviewWindow();

  let prefs = $state<Preferences | null>(null);
  const snapshot = $derived(ui.snapshot);
  const fit = $derived(prefs?.video.fit ?? "cover");
  const fadeMs = $derived(prefs?.video.crossfadeMs ?? 1000);
  const fullscreen = $derived(prefs?.video.fullscreen ?? false);

  // ---- The two playlist layers and the effect layer ----
  let layerEls: HTMLVideoElement[] = $state([]);
  let fxEl = $state<HTMLVideoElement>();
  /** Which layer shows (0 or 1), and what each one has loaded. */
  let active = $state(0);
  let layerVideo = $state<(string | null)[]>([null, null]);
  let layerShown = $state([false, false]);
  let fxShown = $state(false);
  let fxPlaying = $state<string | null>(null);
  /** What each layer (and the effect layer) has loaded, to free it when it's replaced. */
  const loads: (Playback | null)[] = [null, null];
  let fxLoad: Playback | null = null;
  /** Bumped by every show(), so a slow load that a newer one overtook doesn't take over. */
  let showToken = 0;
  /** The layer a fading loop is leaving (see loopWithFade), or -1. While it's still the active
   *  layer, the loop's fade is on its way and mustn't be started again. */
  let loopingFrom = -1;

  /** Where each video stopped (ms), shared with Rust. */
  let positions: Record<string, number> = {};
  /** Per scene: its play order (indexes into its videos) and where it is in it. */
  const sceneStates = new Map<string, { key: string; order: number[]; index: number }>();
  /** The scene and video now showing (null when black). */
  let current = $state<{ sceneId: string; videoId: string } | null>(null);
  /** How long the current video has played in this turn (for "switch every N minutes"). */
  let playedMs = 0;
  let lastTick = performance.now();

  const target = $derived<VideoTarget | null>(snapshot?.project && snapshot.playback.transport !== "stopped" ? snapshot.mix.video : null);
  // Space pauses the sound; the video follows only if "keep playing while paused" is off.
  // (Esc stops everything: `target` is null then, and the window goes black.)
  const paused = $derived(snapshot?.playback.transport === "paused" && !(prefs?.video.keepPlayingWhenPaused ?? true));

  function shuffled(n: number, avoidFirst?: number): number[] {
    const order = [...Array(n).keys()];
    for (let i = n - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [order[i], order[j]] = [order[j], order[i]];
    }
    // Don't play the same video twice in a row across a reshuffle.
    if (n > 1 && order[0] === avoidFirst) [order[0], order[1]] = [order[1], order[0]];
    return order;
  }

  /** The play state of a scene's playlist, rebuilt if the playlist changed. */
  function stateOf(t: VideoTarget) {
    const key = `${t.videos.join(",")}|${t.shuffle}`;
    let st = sceneStates.get(t.sceneId);
    if (!st || st.key !== key) {
      // Keep playing the same video if it's still in the edited playlist.
      const keep = st && current?.sceneId === t.sceneId ? t.videos.indexOf(current.videoId) : -1;
      const order = t.shuffle ? shuffled(t.videos.length) : [...t.videos.keys()];
      const index = keep >= 0 ? order.indexOf(keep) : 0;
      st = { key, order, index: Math.max(0, index) };
      sceneStates.set(t.sceneId, st);
    }
    return st;
  }

  function rememberPosition() {
    const el = layerEls[active];
    const id = layerVideo[active];
    if (!el || !id || !layerShown[active]) return;
    positions[id] = el.ended ? 0 : el.currentTime * 1000;
    api.saveVideoPosition(id, positions[id]).catch(() => {});
  }

  /** Frees a layer once it has faded out (unless it's showing again by then). */
  function release(layer: number) {
    setTimeout(() => {
      if (layerShown[layer]) return;
      layerEls[layer]?.pause();
      loads[layer]?.destroy();
      loads[layer] = null;
      layerVideo[layer] = null;
    }, fadeMs + 50);
  }

  /** Loads `videoId` (at `fromMs`) on the hidden layer and fades it in once it's ready, then
   *  fades the other one out. */
  function show(videoId: string, fromMs: number) {
    const video = videoById(videoId);
    if (!video) return;
    rememberPosition();
    const next = 1 - active;
    const el = layerEls[next];
    if (!el) return;
    const token = ++showToken;
    loads[next]?.destroy();
    layerVideo[next] = videoId;
    const load = loadVideo(el, video, fromMs / 1000);
    loads[next] = load;
    load.ready
      .then(() => {
        if (token !== showToken) return;
        if (!paused && !fxShown) el.play().catch(() => {});
        const old = active;
        active = next;
        layerShown[next] = true;
        layerShown[old] = false;
        release(old);
      })
      .catch((e) => {
        console.warn("video:", videoId, e);
        // A fading loop that couldn't load: loop the old layer the plain way instead.
        if (token === showToken && loopingFrom === active) {
          loopingFrom = -1;
          if (layerEls[active]?.ended) restartActive();
        }
      });
    current = { sceneId: current?.sceneId ?? "", videoId };
    playedMs = 0;
  }

  function blank() {
    rememberPosition();
    showToken++;
    layerShown = [false, false];
    release(0);
    release(1);
    current = null;
  }

  /** Starts (or resumes) the target scene's playlist. */
  function startScene(t: VideoTarget) {
    const st = stateOf(t);
    const videoId = t.videos[st.order[st.index]];
    current = { sceneId: t.sceneId, videoId };
    show(videoId, t.resume ? (positions[videoId] ?? 0) : 0);
    current = { sceneId: t.sceneId, videoId };
  }

  /** The next video of the playlist (switch time, or the video ended). */
  function advance(t: VideoTarget, ended: boolean) {
    const st = stateOf(t);
    if (ended && current) positions[current.videoId] = 0;
    st.index++;
    if (st.index >= st.order.length) {
      st.index = 0;
      if (t.shuffle) st.order = shuffled(t.videos.length, st.order[st.order.length - 1]);
    }
    const videoId = t.videos[st.order[st.index]];
    show(videoId, t.resume ? (positions[videoId] ?? 0) : 0);
    current = { sceneId: t.sceneId, videoId };
  }

  // Another project opened: its videos' positions (kept across restarts) replace these.
  let positionsFor: string | null = null;
  $effect(() => {
    const dir = snapshot?.projectDir ?? null;
    if (dir === positionsFor) return;
    positionsFor = dir;
    if (dir) api.videoPositions().then((p) => (positions = p)).catch(() => {});
  });

  // React to what Rust wants shown. Only `target` is a dependency: `untrack` keeps the layer
  // state read and written by show()/blank() from re-running this effect.
  $effect(() => {
    const t = target;
    untrack(() => {
      if (!t) {
        if (current) blank();
        return;
      }
      const st = stateOf(t);
      const same = current?.sceneId === t.sceneId && t.videos.includes(current.videoId);
      if (!same) startScene(t);
      else st.index = Math.max(0, st.order.indexOf(t.videos.indexOf(current!.videoId)));
    });
  });

  // Space pauses the video along with the sound.
  $effect(() => {
    const el = layerEls[active];
    if (!el) return;
    if (paused || fxShown) el.pause();
    else if (layerShown[active]) el.play().catch(() => {});
  });

  // ---- Video effects ----
  $effect(() => {
    const id = snapshot?.videoEffect ?? null;
    const el = fxEl;
    untrack(() => {
      if (id === fxPlaying || !el) return;
      fxPlaying = id;
      const video = id ? videoById(id) : undefined;
      if (!video) {
        fxShown = false;
        setTimeout(() => {
          if (fxShown) return;
          el.pause();
          fxLoad?.destroy();
          fxLoad = null;
        }, fadeMs + 50);
        return;
      }
      fxLoad?.destroy();
      const load = loadVideo(el, video, 0);
      fxLoad = load;
      load.ready
        .then(() => {
          if (fxPlaying !== id) return;
          el.play().catch(() => {});
          fxShown = true;
        })
        .catch((e) => console.warn("video effect:", id, e));
    });
  });
  $effect(() => {
    if (!fxEl) return;
    if (paused) fxEl.pause();
    else if (fxShown) fxEl.play().catch(() => {});
  });
  function effectEnded() {
    if (fxPlaying) api.videoEffectEnded(fxPlaying).catch(() => {});
  }

  /** How long before its end a lone video starts fading into its start again: the fade, plus
   *  a little for the other layer to load it. */
  const loopLeadMs = $derived(fadeMs + 500);

  /** A lone video loops with a crossfade unless it's marked as looping seamlessly, and it's long
   *  enough that the fade doesn't eat most of it. */
  function fadesWhenLooping(el: HTMLVideoElement, videoId: string): boolean {
    return !videoById(videoId)?.seamlessLoop && el.duration * 1000 > 2 * loopLeadMs;
  }

  /** Starts the video again on the other layer and crossfades into it, so the jump from its end
   *  to its start doesn't show as a hard cut. */
  function loopWithFade(videoId: string) {
    if (loopingFrom === active) return;
    loopingFrom = active;
    show(videoId, 0);
  }

  function restartActive() {
    const el = layerEls[active];
    el.currentTime = 0;
    el.play().catch(() => {});
  }

  function playlistEnded() {
    const t = target;
    if (!t) return;
    if (t.videos.length === 1) {
      // One video loops. A fading loop normally started before the end (in the clock); if it's
      // still loading, the last frame stays until it's ready.
      if (current && fadesWhenLooping(layerEls[active], current.videoId)) loopWithFade(current.videoId);
      else restartActive();
      return;
    }
    advance(t, true);
  }

  // ---- The clock: switching, saving positions, telling the main window ----
  let lastStatus = 0;
  let lastSave = 0;
  const clock = setInterval(() => {
    const now = performance.now();
    const dt = now - lastTick;
    lastTick = now;
    const t = target;
    const el = layerEls[active];
    const playing = !!t && !!current && !paused && !fxShown && !!el && !el.paused;
    if (playing) playedMs += dt;
    const switchMs = t && t.switchMinutes > 0 && t.videos.length > 1 ? t.switchMinutes * 60_000 : null;
    if (t && switchMs !== null && playedMs >= switchMs) advance(t, false);
    if (playing && t.videos.length === 1 && fadesWhenLooping(el, current!.videoId)) {
      if ((el.duration - el.currentTime) * 1000 < loopLeadMs) loopWithFade(current!.videoId);
    }
    if (now - lastSave > 5000) {
      lastSave = now;
      if (playing) rememberPosition();
    }
    if (now - lastStatus > 1000) {
      lastStatus = now;
      emit("video-status", {
        videoId: current?.videoId ?? null,
        sceneId: current?.sceneId ?? null,
        positionMs: el && current ? el.currentTime * 1000 : 0,
        switchInMs: switchMs !== null ? Math.max(0, switchMs - playedMs) : null,
        effect: fxShown ? fxPlaying : null,
      }).catch(() => {});
    }
  }, 250);
  onDestroy(() => {
    clearInterval(clock);
    rememberPosition();
    loads.forEach((l) => l?.destroy());
    fxLoad?.destroy();
    emit("video-status", { videoId: null, sceneId: null, positionMs: 0, switchInMs: null, effect: null }).catch(() => {});
  });

  // ---- Controls: shown while the pointer moves over the window ----
  let controls = $state(true);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  function wake() {
    controls = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => (controls = false), 2500);
  }
  let screens = $state<Screen[]>([]);
  const screenName = $derived(prefs?.video.screen ?? screens.find((s) => s.primary)?.name ?? "");

  function toggleFullscreen() {
    api.setVideoFullscreen(!fullscreen).catch(() => {});
  }

  // Space works as in the main window. Esc leaves fullscreen first, like everywhere else;
  // otherwise it stops everything, as in the main window.
  function onKeydown(e: KeyboardEvent) {
    if (e.key === " ") {
      e.preventDefault();
      api.togglePlay().catch(() => {});
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (fullscreen) toggleFullscreen();
      else api.stopAll().catch(() => {});
    } else if (e.key === "f" || e.key === "F") {
      toggleFullscreen();
    }
  }

  onMount(() => {
    document.documentElement.classList.add("video-window");
    const unlisten = [
      listen<Snapshot>("state-changed", (e) => (ui.snapshot = e.payload)),
      listen<Preferences>("preferences-changed", (e) => (prefs = e.payload)),
    ];
    Promise.all([api.getState(), api.getPreferences(), api.videoPositions(), api.listScreens(), api.videoStreaming()]).then(
      ([s, p, pos, sc, streaming]) => {
        positions = pos;
        prefs = p;
        screens = sc;
        ui.videoStreaming = streaming;
        ui.snapshot = s;
      },
    );
    wake();
    return () => unlisten.forEach((u) => u.then((f) => f()));
  });

  const nowName = $derived(current ? (videoById(current.videoId)?.name ?? "") : "");
  const sceneName = $derived(target ? (snapshot?.project?.scenes.find((s) => s.id === target.sceneId)?.name ?? "") : "");
  // For the thin line along the bottom: how far into this turn (when switching is on).
  let turn = $state(0);
  const turnTimer = setInterval(() => {
    const t = target;
    turn = t && t.switchMinutes > 0 && t.videos.length > 1 ? Math.min(1, playedMs / (t.switchMinutes * 60_000)) : 0;
  }, 1000);
  onDestroy(() => clearInterval(turnTimer));
</script>

<svelte:window onkeydown={onKeydown} />

<div class="stage" role="presentation" onmousemove={wake} style:--fade="{fadeMs}ms" class:hide-cursor={!controls}>
  {#each [0, 1] as i (i)}
    <!-- Muted: a video's sound is never played (decision 0035). -->
    <video
      bind:this={layerEls[i]}
      class:shown={layerShown[i]}
      style:object-fit={fit}
      muted
      playsinline
      onended={() => i === active && playlistEnded()}
    ></video>
  {/each}
  <video bind:this={fxEl} class="fx" class:shown={fxShown} style:object-fit={fit} muted playsinline onended={effectEnded}></video>

  {#if !current && !fxShown && !fullscreen}
    <div class="empty">
      <Film size={28} />
      <span>{snapshot?.playback.transport === "stopped" && snapshot?.mix.video ? "Stopped. Press Space in Comilito to start." : "No video in the tapestry"}</span>
      <span class="small">The top-most scene with videos shows here. Fullscreen hides this text.</span>
    </div>
  {/if}

  <!-- The top bar is the window's drag handle (it has no title bar); double-click: fullscreen. -->
  <div class="bar" class:shown={controls} data-tauri-drag-region role="toolbar" tabindex="-1" ondblclick={toggleFullscreen}>
    <Grip size={16} />
    <div class="now" data-tauri-drag-region>
      <strong data-tauri-drag-region>{fxShown ? "Video effect" : sceneName || "Comilito video"}</strong>
      <span data-tauri-drag-region>
        {#if fxShown}{videoById(fxPlaying ?? "")?.name ?? ""}{:else if nowName}{nowName}{#if layerEls[active]} · {formatDuration((layerEls[active]?.currentTime ?? 0) * 1000)}{/if}{/if}
      </span>
    </div>
    {#if screens.length > 1}
      <label class="screen" title="Which screen the window is on">
        <Monitor size={14} />
        <select value={screenName} onchange={(e) => api.setVideoScreen(e.currentTarget.value).catch(() => {})}>
          {#each screens as s (s.name)}<option value={s.name}>{s.name}{s.primary ? " (main)" : ""}</option>{/each}
        </select>
      </label>
    {/if}
    <button title={fullscreen ? "Leave fullscreen (Esc)" : "Fullscreen (F)"} onclick={toggleFullscreen}>
      {#if fullscreen}<Minimize size={15} />{:else}<Maximize size={15} />{/if}
    </button>
    <button title="Close the video window" onclick={() => win.close()}><X size={15} /></button>
  </div>
  {#if !fullscreen && turn > 0}
    <div class="turn"><i style:width="{turn * 100}%"></i></div>
  {/if}
</div>

<style>
  :global(html.video-window),
  :global(html.video-window body) {
    background: #000;
    overflow: hidden;
  }
  .stage {
    position: fixed;
    inset: 0;
    background: #000;
    color: #fff;
  }
  .stage.hide-cursor {
    cursor: none;
  }
  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    transition: opacity var(--fade) ease;
    pointer-events: none;
  }
  video.shown {
    opacity: 1;
  }
  .fx {
    z-index: 1;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.6rem;
    color: rgb(255 255 255 / 0.45);
  }
  .empty .small {
    font-size: 0.8rem;
    color: rgb(255 255 255 / 0.3);
  }
  .bar {
    position: absolute;
    z-index: 2;
    top: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.9rem 1.1rem 1.6rem;
    background: linear-gradient(to bottom, rgb(0 0 0 / 0.8), transparent);
    opacity: 0;
    transition: opacity 0.3s ease;
    color: rgb(255 255 255 / 0.85);
    user-select: none;
    -webkit-user-select: none;
  }
  .bar.shown {
    opacity: 1;
  }
  .now {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    font-size: 0.8rem;
  }
  .now strong {
    font-size: 0.95rem;
    color: #fff;
  }
  .now span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar button,
  .screen {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: rgb(255 255 255 / 0.14);
    border: none;
    border-radius: 4px;
    color: #fff;
    padding: 0.35rem 0.5rem;
  }
  .screen select {
    background: transparent;
    border: none;
    color: #fff;
    font-size: 0.8rem;
    padding: 0;
  }
  .screen option {
    color: #000;
  }
  .turn {
    position: absolute;
    z-index: 2;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: rgb(255 255 255 / 0.12);
  }
  .turn i {
    display: block;
    height: 100%;
    background: #dd4b2e;
    transition: width 1s linear;
  }
</style>
