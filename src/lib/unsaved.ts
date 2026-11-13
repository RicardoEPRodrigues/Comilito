// Closing an editor (✕ or Esc) with unsaved changes: ask, save or discard, depending on the
// "unsaved changes" preference. The editor's own Cancel button always discards without asking.

import { ask, ui } from "./store.svelte";

/** A string fingerprint of a draft, to compare with the original and see if anything changed. */
export function fingerprint(value: unknown): string {
  return JSON.stringify(value);
}

/**
 * Closes an editor, dealing with unsaved changes first.
 * `save` saves and closes; `discard` just closes.
 */
export async function closeEditor(dirty: boolean, save: () => Promise<void>, discard: () => void) {
  if (!dirty) {
    discard();
    return;
  }
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
  // "Keep editing" or Esc: leave the editor open.
}
