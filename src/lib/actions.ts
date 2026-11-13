// User actions that need a question first (or are used from several components).

import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { ask, askText, project, run, sceneById, soundById, ui } from "./store.svelte";
import type { Category } from "./types";

/**
 * Adds a scene to the stack at `index` (0 = bottom). If it's already in the stack, asks
 * whether to move it or add a duplicate, unless the preferences say which (docs/scenes.md).
 */
export async function insertScene(sceneId: string, index: number) {
  const p = project();
  if (!p) return;
  let mode: "move" | "duplicate" = "duplicate";

  if (p.stack.some((e) => e.scene === sceneId)) {
    const pref = ui.prefs?.behaviour.duplicateInStack ?? "ask";
    if (pref === "ask") {
      const name = sceneById(sceneId)?.name ?? "This scene";
      const answer = await ask({
        title: "Already in the stack",
        message: `${name} is already in the stack.`,
        choices: [
          { label: "Cancel", value: "cancel" },
          { label: "Add a duplicate", value: "duplicate" },
          { label: "Move it here", value: "move", primary: true },
        ],
      });
      if (!answer || answer.choice === "cancel") return;
      mode = answer.choice as "move" | "duplicate";
    } else {
      mode = pref;
    }
  }
  await run(api.stackInsert(sceneId, index, mode));
}

export function addSceneOnTop(sceneId: string) {
  return insertScene(sceneId, project()?.stack.length ?? 0);
}

export async function newScene() {
  const name = await askText("New scene", "Name");
  if (!name) return;
  const id = await run(api.createScene(name, null));
  if (id) ui.editing = { kind: "scene", id };
}

export async function deleteScene(sceneId: string) {
  const p = project();
  const scene = sceneById(sceneId);
  if (!p || !scene) return;
  const presets = p.stackPresets.filter((pr) => pr.stack.some((e) => e.scene === sceneId));
  const inStack = p.stack.some((e) => e.scene === sceneId);
  const notes = [
    inStack ? "It will be removed from the stack." : "",
    presets.length ? `It will be removed from: ${presets.map((x) => x.name).join(", ")}.` : "",
  ].filter(Boolean);

  const answer = await ask({
    title: `Delete "${scene.name}"?`,
    message: notes.join("\n") || undefined,
    choices: [
      { label: "Cancel", value: "cancel" },
      { label: "Delete", value: "delete", danger: true, primary: true },
    ],
  });
  if (answer?.choice === "delete") await run(api.deleteScene(sceneId));
}

/** Opens the system file picker and copies the chosen files into the project. */
export async function addSoundFiles(category: Category) {
  const picked = await open({
    multiple: true,
    title: "Add sounds",
    filters: [{ name: "Audio", extensions: ["mp3", "ogg", "oga", "flac", "wav"] }],
  });
  // `open` returns null when cancelled, otherwise an array of paths (because multiple: true).
  if (!picked || picked.length === 0) return;
  await run(api.addSounds(category, picked));
}

/** Removes a sound, asking whether to trash or delete its file (decision 0007). */
export async function removeSound(soundId: string) {
  const p = project();
  const sound = soundById(soundId);
  if (!p || !sound) return;

  const usedIn = p.scenes
    .filter(
      (s) =>
        s.music?.tracks.includes(soundId) ||
        s.ambience.some((l) => l.sound === soundId) ||
        s.effects.includes(soundId),
    )
    .map((s) => s.name);

  const pref = ui.prefs?.behaviour.removeSound ?? "ask";
  if (pref === "trash" && usedIn.length === 0) {
    await run(api.removeSound(soundId, false));
    return;
  }

  const answer = await ask({
    title: `Remove "${sound.name}"?`,
    message:
      (usedIn.length ? `It's used in: ${usedIn.join(", ")}. It will be removed from them.\n\n` : "") +
      "Its audio file can go to the system trash (you can restore it from there) or be deleted for good.",
    choices: [
      { label: "Cancel", value: "cancel" },
      { label: "Delete permanently", value: "delete", danger: true },
      { label: "Move to trash", value: "trash", primary: true },
    ],
  });
  if (answer?.choice === "trash") await run(api.removeSound(soundId, false));
  if (answer?.choice === "delete") await run(api.removeSound(soundId, true));
}

export async function savePreset() {
  const name = await askText("Save stack as preset", "Preset name");
  if (name) await run(api.presetSave(name));
}

export async function renamePreset(presetId: string, current: string) {
  const name = await askText("Rename preset", "Name", current);
  if (name) await run(api.presetRename(presetId, name));
}
