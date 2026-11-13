<script lang="ts">
  // Draws waveform peaks (from Rust, see waveform.rs) on a canvas. Optionally shows a
  // selection with draggable start/end handles, for cropping (decision 0020).
  //
  // Positions are fractions of the width (0 = left edge, 1 = right edge); the parent converts
  // them to times for the part of the sound it's showing.
  import { onMount } from "svelte";

  interface Props {
    peaks: number[];
    height?: number;
    /** The selected part, as fractions of the width. */
    selection?: { start: number; end: number } | null;
    /** Called while dragging a handle or drawing a new selection. */
    onselect?: (start: number, end: number, handle: "start" | "end" | "new") => void;
    /** Called when the mouse is released after changing the selection. */
    onrelease?: (handle: "start" | "end" | "new") => void;
    /** Where playback is, as a fraction (or null). */
    playhead?: number | null;
    /** Played part shown in the accent colour (simple waveform without selection). */
    progress?: number | null;
    /** A click on a waveform without selection: play from that point. */
    onseek?: (fraction: number) => void;
  }
  let { peaks, height = 56, selection = null, onselect, onrelease, playhead = null, progress = null, onseek }: Props = $props();

  let canvas: HTMLCanvasElement;
  let box: HTMLDivElement;
  let width = $state(600);
  let scheduled = false;

  /** Draws on the next frame, once the browser has laid the canvas out. Drawing straight away
   *  could happen before layout and leave the canvas blank until something else changed. */
  function schedule() {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      draw();
    });
  }
  let dragging: "start" | "end" | "new" | null = null;
  let anchor = 0;

  function colour(name: string) {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "#888";
  }

  function draw() {
    if (!canvas || !box) return;
    width = box.clientWidth || width;
    const ratio = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(width * ratio));
    canvas.height = Math.round(height * ratio);
    const ctx = canvas.getContext("2d")!;
    ctx.scale(ratio, ratio);
    ctx.clearRect(0, 0, width, height);

    const accent = colour("--accent");
    const gold = colour("--gold");
    const text = colour("--text");
    const n = peaks.length || 1;
    const barWidth = Math.max(1, width / n - 1);
    const mid = height / 2;
    for (let i = 0; i < peaks.length; i++) {
      const x = (i / n) * width;
      const f = (i + 0.5) / n;
      const inSelection = selection ? f >= selection.start && f <= selection.end : progress !== null ? f <= progress : false;
      ctx.fillStyle = inSelection ? accent : gold;
      // A little boost so quiet files are still visible.
      const h = Math.max(1.5, Math.min(1, Math.sqrt(peaks[i]) ) * (height - 6));
      ctx.fillRect(x, mid - h / 2, barWidth, h);
    }

    if (selection) {
      // Dim what will be cut away, and draw the two handles.
      ctx.fillStyle = colour("--bg") + "c8";
      ctx.fillRect(0, 0, selection.start * width, height);
      ctx.fillRect(selection.end * width, 0, width - selection.end * width, height);
      ctx.fillStyle = text;
      for (const f of [selection.start, selection.end]) {
        const x = f * width;
        ctx.fillRect(x - 1, 0, 3, height);
        ctx.beginPath();
        ctx.roundRect(x - 8, 0, 17, 20, [0, 0, 4, 4]);
        ctx.fill();
      }
    }
    if (playhead !== null && playhead >= 0 && playhead <= 1) {
      ctx.fillStyle = text;
      ctx.globalAlpha = 0.6;
      ctx.fillRect(playhead * width, 0, 2, height);
      ctx.globalAlpha = 1;
    }
  }

  // Redraw whenever anything drawn changes. Reading the values here makes `$effect` track them.
  $effect(() => {
    void [peaks, peaks.length, selection?.start, selection?.end, playhead, progress, height];
    schedule();
  });

  onMount(() => {
    // Theme changes recolour <html>; redraw when its style changes.
    const observer = new MutationObserver(() => schedule());
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "class"] });
    // And redraw whenever the waveform's box changes size (window resized, panel shown...).
    const resize = new ResizeObserver(() => schedule());
    resize.observe(box);
    schedule();
    return () => {
      observer.disconnect();
      resize.disconnect();
    };
  });

  function fraction(e: PointerEvent) {
    const r = canvas.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  }

  function down(e: PointerEvent) {
    if (!selection && onseek) {
      onseek(fraction(e));
      return;
    }
    if (!selection || !onselect) return;
    const f = fraction(e);
    const near = 10 / width;
    if (Math.abs(f - selection.start) < near) dragging = "start";
    else if (Math.abs(f - selection.end) < near) dragging = "end";
    else {
      dragging = "new";
      anchor = f;
    }
    canvas.setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!selection && onseek) {
      canvas.style.cursor = "pointer";
      return;
    }
    if (!selection || !onselect) return;
    const f = fraction(e);
    if (!dragging) {
      const near = 10 / width;
      canvas.style.cursor = Math.abs(f - selection.start) < near || Math.abs(f - selection.end) < near ? "ew-resize" : "crosshair";
      return;
    }
    if (dragging === "start") onselect(Math.min(f, selection.end - 0.002), selection.end, "start");
    else if (dragging === "end") onselect(selection.start, Math.max(f, selection.start + 0.002), "end");
    else onselect(Math.min(anchor, f), Math.max(anchor, f), "new");
  }

  function up() {
    if (dragging) onrelease?.(dragging);
    dragging = null;
  }
</script>

<div class="wave" bind:this={box} style:height="{height}px">
  <canvas bind:this={canvas} style:width="{width}px" style:height="{height}px" onpointerdown={down} onpointermove={move} onpointerup={up}></canvas>
</div>

<style>
  .wave {
    width: 100%;
    min-width: 0;
  }
  canvas {
    display: block;
    touch-action: none;
  }
</style>
