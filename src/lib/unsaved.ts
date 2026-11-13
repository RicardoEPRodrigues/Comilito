// Leaving an editor with unsaved changes: ask, save or discard, depending on the
// "unsaved changes" preference. Used by the settings windows (✕ / Esc) and by the in-place
// editors when you navigate away (a breadcrumb, another scene, the Library...).

import { ask, ui } from "./store.svelte";

/** A string fingerprint of a draft, to compare with the original and see if anything changed. */
export function fingerprint(value: unknown): string {
  return JSON.stringify(value);
}

/**
 * Decides what happens to unsaved changes. Resolves true if it's fine to leave (saved or
 * discarded), false to keep editing.
 */
export async function confirmLeave(dirty: boolean, save: () => Promise<unknown>): Promise<boolean> {
  if (!dirty) return true;
  const pref = ui.prefs?.behaviour.unsavedChanges ?? "ask";
  if (pref === "save") {
    await save();
    return true;
  }
  if (pref === "discard") return true;

  const answer = await ask({
    title: "Unsaved changes",
    message: "You changed things here. Save them?",
    choices: [
      { label: "Keep editing", value: "keep" },
      { label: "Discard", value: "discard", danger: true },
      { label: "Save", value: "save", primary: true },
    ],
  });
  if (answer?.choice === "save") {
    await save();
    return true;
  }
  return answer?.choice === "discard";
}

/**
 * Closes a settings window, dealing with unsaved changes first.
 * `save` saves and closes; `discard` just closes.
 */
export async function closeEditor(dirty: boolean, save: () => Promise<void>, discard: () => void) {
  if (!dirty) return discard();
  const pref = ui.prefs?.behaviour.unsavedChanges ?? "ask";
  if (pref === "save") return save();
  if (pref === "discard") return discard();
  const answer = await ask({
    title: "Unsaved changes",
    message: "You changed things in this window. Save them?",
    choices: [
      { label: "Keep editing", value: "keep" },
      { label: "Discard", value: "discard", danger: true },
      { label: "Save", value: "save", primary: true },
    ],
  });
  if (answer?.choice === "save") return save();
  if (answer?.choice === "discard") return discard();
}
