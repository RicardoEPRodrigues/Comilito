// The interface scale (Preferences › Appearance; decision 0048). The whole page is zoomed by
// the webview itself, like Ctrl + / Ctrl − in a browser: text, icons, spacing and layout all
// grow together and reflow to the window, and page coordinates (clientX, getBoundingClientRect)
// stay consistent with each other. The only thing that doesn't follow is a position that comes
// from outside the page, in window pixels: see `zoom` below.
import { getCurrentWebview } from "@tauri-apps/api/webview";

/** The steps offered in Preferences and walked through by Ctrl + / Ctrl −. Keep the ends in
 *  step with MIN_SCALE / MAX_SCALE in prefs.rs. */
export const SCALES = [0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2];

/** The zoom the webview has now. Read by code that gets window pixels (the native file drop). */
export let zoom = 1;

/** Zooms the main window's webview. Always sent, even for the zoom `zoom` says it has: a
 *  restart of the interface (F5) reloads the page, which resets `zoom` but not the webview. */
export async function applyScale(scale: number) {
  if (!Number.isFinite(scale) || scale <= 0) return;
  zoom = scale;
  await getCurrentWebview().setZoom(scale);
}

/** The next step up (`direction` 1) or down (-1) from `scale`; it may sit between steps. */
export function stepScale(scale: number, direction: 1 | -1): number {
  if (direction > 0) return SCALES.find((s) => s > scale + 1e-6) ?? SCALES[SCALES.length - 1];
  return [...SCALES].reverse().find((s) => s < scale - 1e-6) ?? SCALES[0];
}
