// Shared helpers for the waveforms in the sound editor and the crop editor (TODO.md):
// zooming with Ctrl + scroll, and the playback cursor.

import { api } from "./api";

export interface View {
  start: number;
  end: number;
}

/**
 * Zooms `view` by `factor` (< 1 zooms in) keeping the point under the pointer (`anchor`,
 * 0–1 across the waveform) in place. Never narrower than half a second, never wider than
 * the whole sound.
 */
export function zoomView(view: View, duration: number, factor: number, anchor = 0.5): View {
  const span = view.end - view.start;
  const newSpan = Math.min(duration, Math.max(500, span * factor));
  const at = view.start + anchor * span;
  let start = at - anchor * newSpan;
  start = Math.max(0, Math.min(start, duration - newSpan));
  return { start, end: start + newSpan };
}

/** Moves the view sideways by `fraction` of its width (plain scroll while zoomed in). */
export function panView(view: View, duration: number, fraction: number): View {
  const span = view.end - view.start;
  const start = Math.max(0, Math.min(view.start + fraction * span, duration - span));
  return { start, end: start + span };
}

/**
 * The wheel handler for a waveform: Ctrl + scroll zooms around the pointer, plain scroll
 * pans when zoomed in. Returns the new view, or null if the event isn't ours.
 */
export function wheelView(e: WheelEvent, view: View, duration: number): View | null {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const anchor = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
  const delta = e.deltaY !== 0 ? e.deltaY : e.deltaX;
  if (e.ctrlKey || e.metaKey) {
    e.preventDefault();
    return zoomView(view, duration, delta > 0 ? 1.25 : 0.8, anchor);
  }
  if (view.end - view.start < duration) {
    e.preventDefault();
    return panView(view, duration, delta > 0 ? 0.1 : -0.1);
  }
  return null;
}

/**
 * Follows the preview of one sound: calls `onPosition` with its position in ms (or null when
 * it isn't playing) about ten times a second. Returns a function that stops following.
 */
export function followPreview(soundId: string, onPosition: (ms: number | null) => void): () => void {
  let stopped = false;
  const tick = async () => {
    if (stopped) return;
    try {
      const pos = await api.previewPosition();
      onPosition(pos && pos[0] === soundId ? pos[1] : null);
    } catch {
      onPosition(null);
    }
    if (!stopped) setTimeout(tick, 100);
  };
  tick();
  return () => {
    stopped = true;
  };
}
