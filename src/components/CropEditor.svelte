<script lang="ts">
  // Cropping a sound (decision 0020). Drag the handles on the waveform (zoom in for precision)
  // or type exact times; preview the selection or the loop seam; then save a copy (default)
  // or overwrite the original, whose file goes to the system trash. ffmpeg does the cutting.
  import { Play, Repeat, Scan, Scissors, Square, ZoomIn, ZoomOut } from "@lucide/svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { formatBytes, notify, refreshTools, run, ui } from "../lib/store.svelte";
  import type { Sound } from "../lib/types";
  import Capital from "./Capital.svelte";
  import ToolsInstall from "./ToolsInstall.svelte";
  import Waveform from "./Waveform.svelte";
  import { followPreview, wheelView, zoomView } from "../lib/wave";

  let { sound }: { sound: Sound } = $props();

  let duration = $state(0);
  let view = $state({ start: 0, end: 0 });
  let sel = $state({ start: 0, end: 0 });
  let peaks = $state<number[]>([]);
  let width = $state(700);
  let loading = $state(true);
  let fadeEdges = $state(true);
  let snap = $state(true);
  let exact = $state(false);
  let overwrite = $state(false);
  let saving = $state(false);
  const previewing = $derived(ui.snapshot?.playback.preview === sound.id);
  const ext = $derived(sound.file.path.split(".").pop()?.toUpperCase() ?? "");

  let position = $state<number | null>(null);
  onMount(() => followPreview(sound.id, (ms) => (position = ms)));
  const cursor = $derived(position !== null && view.end > view.start ? (position - view.start) / (view.end - view.start) : null);

  onMount(async () => {
    refreshTools();
    try {
      const full = await api.soundWaveform(sound.id, 10);
      duration = full.durationMs;
      view = { start: 0, end: duration };
      sel = { start: 0, end: duration };
    } catch (e) {
      notify(String(e));
    }
    loading = false;
  });

  // Fetch peaks for the visible range whenever it (or the width) changes.
  $effect(() => {
    const { start, end } = view;
    const buckets = Math.max(50, Math.round(width / 3));
    if (end <= start) return;
    api.soundWaveform(sound.id, buckets, start, end).then((w) => {
      if (view.start === start && view.end === end) peaks = w.peaks;
    });
  });

  const toFraction = (ms: number) => (ms - view.start) / (view.end - view.start || 1);
  const toMs = (f: number) => view.start + f * (view.end - view.start);

  function onSelect(start: number, end: number) {
    sel = { start: Math.round(toMs(start)), end: Math.round(toMs(end)) };
  }

  /** Moves a cut to the quietest moment within a quarter second, if "snap" is on. */
  function snapToQuiet(ms: number): number {
    if (!snap || peaks.length === 0) return ms;
    const bucketMs = (view.end - view.start) / peaks.length;
    const reach = Math.max(1, Math.round(250 / bucketMs));
    const centre = Math.round((ms - view.start) / bucketMs);
    let best = centre;
    for (let i = Math.max(0, centre - reach); i <= Math.min(peaks.length - 1, centre + reach); i++) {
      if (peaks[i] < (peaks[best] ?? Infinity)) best = i;
    }
    return Math.round(view.start + (best + 0.5) * bucketMs);
  }

  function onRelease(handle: "start" | "end" | "new") {
    if (handle === "start" || handle === "new") sel.start = Math.max(0, snapToQuiet(sel.start));
    if (handle === "end" || handle === "new") sel.end = Math.min(duration, snapToQuiet(sel.end));
    if (sel.end <= sel.start) sel.end = Math.min(duration, sel.start + 100);
  }

  /** The zoom buttons zoom around the middle of the selection. */
  function zoom(factor: number) {
    const centre = (sel.start + sel.end) / 2;
    const anchor = Math.min(1, Math.max(0, (centre - view.start) / (view.end - view.start || 1)));
    view = zoomView(view, duration, factor, anchor);
  }

  /** "m:ss.cc" (or "h:mm:ss.cc", or plain seconds) → milliseconds. */
  function parseTime(text: string): number | null {
    const parts = text.trim().split(":").map(Number);
    if (parts.some((p) => Number.isNaN(p))) return null;
    const seconds = parts.reduce((acc, p) => acc * 60 + p, 0);
    return Math.round(seconds * 1000);
  }

  function formatTime(ms: number): string {
    const total = ms / 1000;
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = (total % 60).toFixed(2).padStart(5, "0");
    return h ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
  }

  function setField(which: "start" | "end" | "length", text: string) {
    const ms = parseTime(text);
    if (ms === null) return;
    if (which === "start") sel.start = Math.min(Math.max(0, ms), sel.end - 100);
    if (which === "end") sel.end = Math.max(Math.min(duration, ms), sel.start + 100);
    if (which === "length") sel.end = Math.min(duration, sel.start + Math.max(100, ms));
  }

  function preview(seam: boolean) {
    if (previewing) run(api.stopPreview());
    else run(api.previewRange(sound.id, sel.start, sel.end, seam));
  }

  async function save() {
    saving = true;
    const result = await run(api.cropSound(sound.id, { startMs: sel.start, endMs: sel.end, fadeEdges, exact }, overwrite));
    saving = false;
    if (!result) return;
    notify(overwrite ? `Cropped “${sound.name}”.` : `Saved “${sound.name} (cropped)”.`);
    ui.library.cropping = false;
    ui.library.selected = [result];
  }

  const ruler = $derived(Array.from({ length: 6 }, (_, i) => view.start + ((view.end - view.start) * i) / 5));
  const keptFraction = $derived(duration ? (sel.end - sel.start) / duration : 1);
</script>

<div class="editor">
  <div class="head">
    <Capital name={sound.name} size={58} live />
    <div class="title">
      <h2 class="display">{sound.name}</h2>
      <span class="caps muted">Crop · {ext} · {formatTime(duration)} · {formatBytes(sound.file.size)}</span>
    </div>
  </div>

  {#if ui.tools && !ui.tools.ffmpeg}
    <ToolsInstall reason="Cropping uses ffmpeg, which isn't installed yet." />
  {/if}

  <div class="section-head">
    <span class="caps">Crop</span>
    <span class="hint">Drag the handles or across the waveform · Ctrl + scroll to zoom</span>
    <button class="icon framed" title="Zoom out" onclick={() => zoom(2)}><ZoomOut size={15} /></button>
    <button class="icon framed" title="Zoom in on the selection" onclick={() => zoom(0.4)}><ZoomIn size={15} /></button>
    <button class="icon framed" title="Show everything" onclick={() => (view = { start: 0, end: duration })}><Scan size={15} /></button>
  </div>

  <div
    class="wave"
    bind:clientWidth={width}
    role="presentation"
    onwheel={(e) => {
      const next = wheelView(e, view, duration);
      if (next) view = next;
    }}
  >
    {#if loading}
      <p class="muted">Reading the audio…</p>
    {:else}
      <Waveform
        {peaks}
        height={150}
        selection={{ start: Math.max(0, toFraction(sel.start)), end: Math.min(1, toFraction(sel.end)) }}
        onselect={onSelect}
        onrelease={onRelease}
        playhead={cursor}
      />
    {/if}
  </div>
  <div class="ruler">
    {#each ruler as t, i (i)}<span>{formatTime(t)}</span>{/each}
  </div>

  <div class="fields">
    <label class="field"><span class="caps">Start</span><input value={formatTime(sel.start)} onchange={(e) => setField("start", e.currentTarget.value)} /></label>
    <label class="field"><span class="caps">End</span><input value={formatTime(sel.end)} onchange={(e) => setField("end", e.currentTarget.value)} /></label>
    <label class="field"><span class="caps">Length</span><input value={formatTime(sel.end - sel.start)} onchange={(e) => setField("length", e.currentTarget.value)} /></label>
    <span class="grow"></span>
    <button onclick={() => preview(false)}>{#if previewing}<Square size={13} />Stop{:else}<Play size={13} />Preview{/if}</button>
    <button onclick={() => preview(true)} title="Play the end into the start, as a loop would"><Repeat size={13} />Loop seam</button>
  </div>

  <div class="options">
    <label class="switch"><input type="checkbox" bind:checked={fadeEdges} /><span><strong>Fade edges</strong><small>10 ms, avoids clicks</small></span></label>
    <label class="switch"><input type="checkbox" bind:checked={snap} /><span><strong>Snap to quiet points</strong><small>nearest low-volume moment</small></span></label>
    <label class="switch"><input type="checkbox" bind:checked={exact} /><span><strong>Keep exact cut</strong><small>re-encodes the audio</small></span></label>
  </div>

  <div class="choices">
    <label class="choice" class:selected={!overwrite}>
      <input type="radio" bind:group={overwrite} value={false} />
      <span><strong>Save as a copy</strong><small>New sound “{sound.name} (cropped)” in the same folder. Scenes keep using the original.</small></span>
    </label>
    <label class="choice" class:selected={overwrite}>
      <input type="radio" bind:group={overwrite} value={true} />
      <span><strong>Overwrite the original</strong><small>Every scene using it gets the cropped version. The original file goes to the system trash.</small></span>
    </label>
  </div>

  <div class="actions">
    <span class="muted small">
      Keeps {Math.round(keptFraction * 100)}% · {fadeEdges || exact ? "re-encoded at high quality" : "copied without re-encoding, same quality"}
    </span>
    <span class="grow"></span>
    <button class="ghost" onclick={() => (ui.library.cropping = false)}>Cancel</button>
    <button class="primary" onclick={save} disabled={saving || !ui.tools?.ffmpeg || sel.end - sel.start < 100}>
      <Scissors size={14} />{saving ? "Cropping…" : overwrite ? "Overwrite" : "Save copy"}
    </button>
  </div>
</div>

<style>
  .editor {
    flex: 1;
    overflow: auto;
    padding: 1.3rem 2rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .title {
    min-width: 0;
  }
  h2 {
    font-size: 2.3rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title .caps {
    font-size: 0.6rem;
  }
  .framed {
    border: 1px solid var(--rule) !important;
  }
  .section-head .hint {
    margin-right: 0.4rem;
  }
  .wave {
    background: var(--surface);
    border: 1px solid var(--rule);
    border-radius: var(--radius);
    overflow: hidden;
    min-height: 152px;
  }
  .wave p {
    padding: 1rem;
    margin: 0;
  }
  .ruler {
    display: flex;
    justify-content: space-between;
    font-size: 0.68rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    margin-top: -0.6rem;
  }
  .fields {
    display: flex;
    align-items: flex-end;
    gap: 0.8rem;
  }
  .fields input {
    width: 8rem;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .grow {
    flex: 1;
  }
  .options {
    display: flex;
    gap: 1.6rem;
    flex-wrap: wrap;
  }
  .options span,
  .choice span {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  small {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.8rem;
  }
  .choice {
    display: flex;
    gap: 0.7rem;
    align-items: flex-start;
    padding: 0.8rem;
    border: 1px solid var(--rule);
    border-radius: 6px;
    background: var(--surface);
    cursor: pointer;
  }
  .choice.selected {
    border-color: var(--accent);
    border-width: 2px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .small {
    font-size: 0.78rem;
  }
</style>
