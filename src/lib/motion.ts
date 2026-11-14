// Short transitions for opening and closing things (menus, dialogs, folds, the Library drawer).
// They're Svelte's own (`svelte/transition`), wrapped so every one honours the system's
// "reduce motion" setting and the timings stay the same everywhere.
import { fade, fly, scale, slide } from "svelte/transition";

/** True when the desktop asks for less animation; then everything appears at once. */
function reduced(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const ms = (duration: number) => (reduced() ? 0 : duration);

/** Folds and unfolds (groups, folders, an expanded tapestry entry). */
export function fold(node: Element) {
  return slide(node, { duration: ms(160) });
}

/** Menus and pop-ups: a quick grow from almost full size. */
export function pop(node: Element) {
  return scale(node, { duration: ms(110), start: 0.96, opacity: 0 });
}

/** The dark backdrop behind a dialog. */
export function backdrop(node: Element) {
  return fade(node, { duration: ms(140) });
}

/** A dialog or window rising slightly into place. */
export function rise(node: Element) {
  return fly(node, { duration: ms(170), y: 10, opacity: 0 });
}
