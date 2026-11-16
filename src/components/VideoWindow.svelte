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
    // The last second counts as the end: coming back there would show a moment and loop.
    positions[id] = el.ended || el.currentTime > el.duration - 1 ? 0 : el.currentTime * 1000;
    api.saveVideoPosition(id, positions[id]).catch(() => {});
  }

  /** Frees a layer once it has faded out, unless it's showing again by then or a newer video
   *  started loading on it meanwhile (that load isn't shown yet, and freeing it would leave
   *  the window waiting for it forever). */
  function release(layer: number) {
    const load = loads[layer];
    setTimeout(() => {
      if (layerShown[layer] || loads[layer] !== load) return;
      layerEls[layer]?.pause();
      load?.destroy();
      loads[layer] = null;
      layerVideo[layer] = null;
    }, fadeMs + 50);
  }

  /** Writes a problem with the video window to the log (comilito.log, "See logs"): its console
   *  isn't kept anywhere, and "it stopped" is hard to look into without a trace. */
  function logProblem(message: string) {
    console.warn("video window:", message);
    api.logMessage("warn", `Video window: ${message}`).catch(() => {});
  }

  /** The state of a <video> element, for the log. */
  function describe(el: HTMLVideoElement | undefined, videoId: string | null): string {
    if (!el) return "no element";
    const name = videoId ? (videoById(videoId)?.name ?? videoId) : "nothing";
    const ranges = Array.from({ length: el.buffered.length }, (_, i) => `${el.buffered.start(i).toFixed(1)}-${el.buffered.end(i).toFixed(1)}`);
    const error = el.error ? `, error ${el.error.code} ${el.error.message}` : "";
    return (
      `“${name}” at ${el.currentTime.toFixed(1)}/${el.duration.toFixed(1)} s, readyState ${el.readyState}, ` +
      `networkState ${el.networkState}, ${el.paused ? "paused" : "playing"}${el.ended ? ", ended" : ""}, ` +
      `buffered [${ranges.join(", ")}]${error}`
    );
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
    loadingSince = performance.now();
    loads[next]?.destroy();
    layerVideo[next] = videoId;
    const load = loadVideo(el, video, fromMs / 1000);
    loads[next] = load;
    load.ready
      .then(() => {
        if (token !== showToken) return;
        loadingSince = 0;
        if (!paused && !fxShown) el.play().catch(() => {});
        const old = active;
        active = next;
        layerShown[next] = true;
        layerShown[old] = false;
        release(old);
      })
      .catch((e) => {
        if (token === showToken) loadingSince = 0;
        logProblem(`couldn't load “${video.name}”: ${e instanceof Error ? e.message : (e?.message ?? e)}`);
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
    loadingSince = 0;
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
        // As release(): not if another effect started loading meanwhile.
        const load = fxLoad;
        setTimeout(() => {
          if (fxShown || fxLoad !== load) return;
          el.pause();
          load?.destroy();
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
        .catch((e) => {
          logProblem(`couldn't load the video effect “${video.name}”: ${e instanceof Error ? e.message : (e?.message ?? e)}`);
          // Tell Rust it's over, or the effect would count as playing (and hide the playlist) for good.
          if (fxPlaying === id) effectEnded();
        });
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
    // show() starts a new turn; a loop is the same turn going on, so its time is kept.
    const played = playedMs;
    show(videoId, 0);
    playedMs = played;
  }

  /** How long each video's turn lasts (ms) when the playlist switches every N minutes, else null. */
  function switchMsOf(t: VideoTarget): number | null {
    return t.switchMinutes > 0 && t.videos.length > 1 ? t.switchMinutes * 60_000 : null;
  }

  /** Whether the showing video starts over when it reaches its end, rather than moving on: a
   *  lone video always does; in a playlist that switches every N minutes, a video shorter than
   *  that loops until its turn is over. (Not if the turn ends about when the video does: the
   *  switch would cut into the loop right away.) */
  function loopsAtEnd(t: VideoTarget, el: HTMLVideoElement): boolean {
    if (t.videos.length === 1) return true;
    const switchMs = switchMsOf(t);
    if (switchMs === null) return false;
    const leftMs = Math.max(0, (el.duration - el.currentTime) * 1000) || 0;
    return playedMs + leftMs + loopLeadMs < switchMs;
  }

  function restartActive() {
    const el = layerEls[active];
    el.currentTime = 0;
    el.play().catch(() => {});
  }

  function playlistEnded() {
    const t = target;
    if (!t) return;
    if (loopsAtEnd(t, layerEls[active])) {
      // One video loops (or one shorter than its turn). A fading loop normally started before the end (in the clock); if it's
      // still loading, the last frame stays until it's ready.
      if (current && fadesWhenLooping(layerEls[active], current.videoId)) loopWithFade(current.videoId);
      else restartActive();
      return;
    }
    advance(t, true);
  }

  // ---- The watchdog: a video that should be moving but isn't ----
  // If the shown video's time stands still while it should play (a stream that stopped
  // feeding, a decoder error), it's logged and started again where it was. If a video being
  // loaded never gets to its first frame, it's logged and started from the beginning. At most
  // a few times in a row.
  const STUCK_MS = 4000;
  const LOAD_MS = 8000;
  const MAX_RESTARTS = 3;
  let watchedTime = -1;
  let stillSince = 0;
  let restarts = 0;
  /** When the latest show() started loading (0 once it's shown or failed). */
  let loadingSince = 0;

  function watch(now: number, el: HTMLVideoElement | undefined) {
    if (loadingSince && current && target && !document.hidden && now - loadingSince > LOAD_MS) {
      loadingSince = 0;
      const loading = 1 - active;
      const state = describe(layerEls[loading], layerVideo[loading]);
      if (restarts >= MAX_RESTARTS) {
        logProblem(`a video still won't load after ${MAX_RESTARTS} tries; giving up. ${state}`);
        return;
      }
      restarts++;
      logProblem(`a video took over ${LOAD_MS / 1000} s to load; starting it from the beginning. ${state}`);
      loopingFrom = -1;
      show(current.videoId, 0);
      return;
    }
    const shouldMove = !!target && !!current && !paused && !fxShown && !!el && layerShown[active] && !document.hidden;
    if (!shouldMove || el.currentTime !== watchedTime) {
      if (shouldMove) restarts = 0; // It moved: things are fine again.
      watchedTime = el?.currentTime ?? -1;
      stillSince = now;
      return;
    }
    if (now - stillSince < STUCK_MS) return;
    stillSince = now;
    const state = describe(el, layerVideo[active]) + (loopingFrom === active ? "; its loop was still loading" : "");
    if (restarts >= MAX_RESTARTS) {
      if (restarts++ === MAX_RESTARTS) logProblem(`the video is still stuck after ${MAX_RESTARTS} restarts; giving up. ${state}`);
      return;
    }
    restarts++;
    logProblem(`the video stopped moving; starting it again. ${state}`);
    loopingFrom = -1;
    show(current!.videoId, el.ended ? 0 : el.currentTime * 1000);
  }

  // ---- The clock: switching, saving positions, telling the main window ----
  let lastStatus = 0;
  let lastSave = 0;
  /** Where the showing video is (ms), for the top bar. The element's `currentTime` isn't
   *  reactive (Svelte can't see a DOM property change), so the clock copies it here. */
  let positionMs = $state(0);
  const clock = setInterval(() => {
    const now = performance.now();
    const dt = now - lastTick;
    lastTick = now;
    const t = target;
    const el = layerEls[active];
    const playing = !!t && !!current && !paused && !fxShown && !!el && !el.paused;
    if (playing) playedMs += dt;
    positionMs = el && current ? el.currentTime * 1000 : 0;
    const switchMs = t ? switchMsOf(t) : null;
    if (t && switchMs !== null && playedMs >= switchMs) advance(t, false);
    if (playing && fadesWhenLooping(el, current!.videoId) && loopsAtEnd(t, el)) {
      if ((el.duration - el.currentTime) * 1000 < loopLeadMs) loopWithFade(current!.videoId);
    }
    watch(now, el);
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

  /** Shown under the top bar for a while when Wayland wouldn't let the window move. */
  let screenHint = $state("");
  let screenHintTimer: ReturnType<typeof setTimeout> | undefined;

  async function chooseScreen(name: string) {
    const moved = await api.setVideoScreen(name).catch(() => true);
    clearTimeout(screenHintTimer);
    screenHint = moved ? "" : `This desktop doesn't let apps move their windows. Drag this window to ${name}, or press F: fullscreen goes to ${name}.`;
    if (screenHint) screenHintTimer = setTimeout(() => (screenHint = ""), 8000);
  }

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
    } else if (e.key === prefs?.shortcuts.restartInterface && !e.ctrlKey && !e.altKey && !e.metaKey) {
      // Here it restarts only this window; the main window's shortcut restarts both.
      e.preventDefault();
      location.reload();
    } else if (e.key === "f" || e.key === "F") {
      toggleFullscreen();
    }
  }

  onMount(() => {
    document.documentElement.classList.add("video-window");
    const unlisten = [
      listen<Snapshot>("state-changed", (e) => (ui.snapshot = e.payload)),
      listen<Preferences>("preferences-changed", (e) => (prefs = e.payload)),
      // The main window's "Restart the interface" (decision 0044) restarts this one too.
      listen("restart-interface", () => location.reload()),
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
    const switchMs = t ? switchMsOf(t) : null;
    turn = switchMs !== null ? Math.min(1, playedMs / switchMs) : 0;
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
        {#if fxShown}{videoById(fxPlaying ?? "")?.name ?? ""}{:else if nowName}{nowName}{#if layerEls[active]} · {formatDuration(positionMs)}{/if}{/if}
      </span>
    </div>
    {#if screens.length > 1}
      <label class="screen" title="Which screen the window is on">
        <Monitor size={14} />
        <select value={screenName} onchange={(e) => chooseScreen(e.currentTarget.value)}>
          {#each screens as s (s.name)}<option value={s.name}>{s.name} · {s.width}×{s.height}{s.primary ? " (main)" : ""}</option>{/each}
        </select>
      </label>
    {/if}
    <button title={fullscreen ? "Leave fullscreen (Esc)" : "Fullscreen (F)"} onclick={toggleFullscreen}>
      {#if fullscreen}<Minimize size={15} />{:else}<Maximize size={15} />{/if}
    </button>
    <button title="Close the video window" onclick={() => win.close()}><X size={15} /></button>
  </div>
  {#if screenHint}<div class="screen-hint" role="status">{screenHint}</div>{/if}
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
  /* The list that drops down is drawn by the system, which ignored the bar's colours and
     showed black text on a dark background. Colours of its own, and `color-scheme: dark` so
     the system draws it dark to match. */
  .screen select {
    color-scheme: dark;
    background: transparent;
    border: none;
    color: #fff;
    font-size: 0.8rem;
    padding: 0;
  }
  .screen option {
    background: #222;
    color: #fff;
  }
  .screen-hint {
    position: absolute;
    z-index: 2;
    top: 3.6rem;
    right: 1.1rem;
    max-width: 26rem;
    padding: 0.5rem 0.7rem;
    border-radius: 4px;
    background: rgb(0 0 0 / 0.8);
    color: #fff;
    font-size: 0.8rem;
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
