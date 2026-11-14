// User actions that need a question first, or are used from several components.

import { open, save } from "@tauri-apps/plugin-dialog";
import { askSaveTapestry } from "../components/SaveTapestryDialog.svelte";
import { api } from "./api";
import {
  ask,
  askText,
  confirmAction,
  focusOn,
  presetById,
  project,
  run,
  sceneById,
  scenesUsing,
  soundById,
  ui,
} from "./store.svelte";
import type { Category } from "./types";

export const AUDIO_EXTENSIONS = ["mp3", "ogg", "oga", "flac", "wav", "m4a", "aac"];
const COMILITO_FILTER = [{ name: "Comilito project", extensions: ["comilito"] }];

// ---- The tapestry (stack) ---------------------------------------------------------------------

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
        title: "Already in the tapestry",
        message: `${name} is already in the tapestry.`,
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

/**
 * Before replacing the tapestry (loading a preset, switching to one scene, clearing): if
 * it isn't saved in a preset, offer to save it first (decision 0017). Resolves false to cancel.
 */
async function okToReplaceTapestry(targetName: string): Promise<boolean> {
  const status = ui.snapshot?.stackStatus;
  if (status !== "modified" && status !== "unsaved") return true;
  if (ui.prefs?.behaviour.replaceUnsavedTapestry === "replace") return true;
  const answer = await askSaveTapestry(targetName);
  return answer !== "cancel";
}

export async function loadPreset(presetId: string) {
  const preset = presetById(presetId);
  if (!preset) return;
  if (await okToReplaceTapestry(`“${preset.name}”`)) await run(api.presetLoad(presetId));
}

export async function switchTapestryTo(sceneId: string) {
  const scene = sceneById(sceneId);
  if (!scene) return;
  if (await okToReplaceTapestry(`“${scene.name}” on its own`)) await run(api.stackSwitch(sceneId));
}

export async function clearTapestry() {
  if (!project()?.stack.length) return;
  if (!(await okToReplaceTapestry("an empty tapestry"))) return;
  await run(api.stackClear());
}

/** "Save as new preset…": asks for a name (and group), then saves. */
export async function saveTapestryAsPreset() {
  const answer = await askSaveTapestry(null);
  return answer === "saved";
}

export async function updateOriginPreset() {
  const origin = project()?.stackOrigin;
  if (origin) await run(api.presetUpdate(origin));
}

export async function revertToOrigin() {
  const p = project();
  const origin = p?.stackOrigin ? presetById(p.stackOrigin) : undefined;
  if (!origin) return;
  const ok = await confirmAction(
    `Revert to “${origin.name}”?`,
    "The tapestry goes back to the preset as it was saved. Your changes since are lost.",
    "Revert",
    true,
  );
  if (ok) await run(api.presetLoad(origin.id));
}

// ---- Scenes -----------------------------------------------------------------------------------

export async function newScene(group: string | null = null) {
  const name = await askText("New scene", "Name");
  if (!name) return;
  const id = await run(api.createScene(name, group));
  if (id) await focusOn({ kind: "scene", id });
}

export async function duplicateScene(sceneId: string) {
  const id = await run(api.duplicateScene(sceneId));
  if (id) await focusOn({ kind: "scene", id });
}

export async function deleteScene(sceneId: string) {
  const p = project();
  const scene = sceneById(sceneId);
  if (!p || !scene) return;
  const presets = p.stackPresets.filter((pr) => pr.stack.some((e) => e.scene === sceneId));
  const inStack = p.stack.some((e) => e.scene === sceneId);
  const notes = [
    inStack ? "It will be removed from the tapestry." : "",
    presets.length ? `It will be removed from: ${presets.map((x) => x.name).join(", ")}.` : "",
  ].filter(Boolean);

  const ok = await confirmAction(`Delete “${scene.name}”?`, notes.join("\n") || "This can't be undone.", "Delete", true);
  if (!ok) return;
  if (ui.focus?.kind === "scene" && ui.focus.id === sceneId) {
    ui.guard = null;
    ui.focus = null;
  }
  await run(api.deleteScene(sceneId));
}

// ---- Presets ----------------------------------------------------------------------------------

export async function renamePreset(presetId: string) {
  const preset = presetById(presetId);
  if (!preset) return;
  const name = await askText("Rename preset", "Name", preset.name);
  if (name) await run(api.presetRename(presetId, name));
}

export async function deletePreset(presetId: string) {
  const preset = presetById(presetId);
  if (!preset) return;
  const ok = await confirmAction(`Delete “${preset.name}”?`, "The scenes in it are kept.", "Delete", true);
  if (!ok) return;
  if (ui.focus?.kind === "preset" && ui.focus.id === presetId) {
    ui.guard = null;
    ui.focus = null;
  }
  await run(api.presetDelete(presetId));
}

export async function newPresetGroup(): Promise<string | null> {
  const name = await askText("New group", "Name");
  if (name) await run(api.presetGroupCreate(name));
  return name;
}

export async function movePresetToNewGroup(presetId: string) {
  const name = await askText("New group", "Name");
  if (name) await run(api.presetSetGroup(presetId, name));
}

export async function renamePresetGroup(group: string) {
  const name = await askText("Rename group", "Name", group);
  if (name && name !== group) await run(api.presetGroupRename(group, name));
}

export async function deletePresetGroup(group: string) {
  const ok = await confirmAction(`Delete the group “${group}”?`, "Its presets are kept, ungrouped.", "Delete group", true);
  if (ok) await run(api.presetGroupDelete(group));
}

export async function renameSceneGroup(group: string) {
  const name = await askText("Rename group", "Name", group);
  if (name && name !== group) await run(api.sceneGroupRename(group, name));
}

export async function ungroupScenes(group: string) {
  const ok = await confirmAction(`Ungroup “${group}”?`, "Its scenes are kept, without a group.", "Ungroup", false);
  if (ok) await run(api.sceneGroupUngroup(group));
}

export async function moveSceneToNewGroup(sceneId: string) {
  const name = await askText("New group", "Name");
  if (name) await run(api.sceneSetGroup(sceneId, name));
}

// ---- Sounds -----------------------------------------------------------------------------------

/** Opens the system file picker and copies the chosen files into the project. */
export async function addSoundFiles(category: Category, folder: string | null = null) {
  const picked = await open({
    multiple: true,
    title: "Add sounds",
    filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
  });
  // `open` returns null when cancelled, otherwise an array of paths (because multiple: true).
  if (!picked || picked.length === 0) return;
  await run(api.addSounds(category, picked, folder));
}

/** Removes a sound, asking whether to trash or delete its file (decision 0007). */
export async function removeSound(soundId: string): Promise<boolean> {
  const sound = soundById(soundId);
  if (!sound) return false;
  const usedIn = scenesUsing(soundId).map((s) => s.name);

  const pref = ui.prefs?.behaviour.removeSound ?? "ask";
  if (pref === "trash" && usedIn.length === 0) {
    await run(api.removeSound(soundId, false));
    return true;
  }

  const answer = await ask({
    title: `Remove “${sound.name}”?`,
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
  return answer?.choice === "trash" || answer?.choice === "delete";
}

/**
 * Moves sounds to another category and/or folder, first warning about scenes they would
 * leave (a music sound turned into an effect leaves the playlists it was in).
 */
export async function moveSounds(soundIds: string[], category: Category, folder: string): Promise<boolean> {
  const p = project();
  if (!p) return false;
  const losing = p.scenes
    .filter((s) =>
      soundIds.some(
        (id) =>
          soundById(id)?.category !== category &&
          ((category !== "music" && s.music?.tracks.includes(id)) ||
            (category !== "ambience" && s.ambience.some((l) => l.sound === id)) ||
            (category !== "effects" && s.effects.includes(id))),
      ),
    )
    .map((s) => s.name);

  if (losing.length > 0) {
    const ok = await confirmAction(
      "Change category?",
      `${soundIds.length === 1 ? "This sound" : "These sounds"} will be removed from: ${losing.join(", ")}.`,
      "Change category",
    );
    if (!ok) return false;
  }
  await run(api.moveSounds(soundIds, category, folder));
  return true;
}

/** Moves a Library folder (with everything in it) into `parent` ("" = the top of its
 *  category). The folder keeps its name; Rust refuses a clash or a move into itself. */
export async function moveFolder(category: Category, folder: string, parent: string) {
  const name = folder.split("/").pop()!;
  const to = parent ? `${parent}/${name}` : name;
  if (to === folder) return;
  await run(api.renameFolder(category, folder, to));
  // Keep the moved folder (or a folder inside it) selected at its new place.
  const lib = ui.library;
  if (lib.category === category && lib.folder !== null && (lib.folder === folder || lib.folder.startsWith(`${folder}/`))) {
    lib.folder = to + lib.folder.slice(folder.length);
  }
}

/** Where `folder` can move to: the top of its category and every folder that isn't the folder
 *  itself, inside it, or already its parent. */
export function moveTargets(category: Category, folder: string): string[] {
  const parent = folder.split("/").slice(0, -1).join("/");
  const folders = ui.snapshot?.folders[category] ?? [];
  return ["", ...folders].filter((f) => f !== parent && f !== folder && !f.startsWith(`${folder}/`));
}

export async function newFolder(category: Category, parent: string | null = null) {
  const name = await askText("New folder", "Name");
  if (!name) return;
  await run(api.createFolder(category, parent ? `${parent}/${name}` : name));
}

// ---- Projects ---------------------------------------------------------------------------------

/** Export as a .comilito zip, asking first whether to include the audio. */
export async function exportProject() {
  const p = project();
  if (!p) return;
  const answer = await ask({
    title: "Export project",
    message:
      "With audio: one self-contained file.\nWithout audio: a small file with only the settings; " +
      "on import you'll be asked where the audio is.",
    choices: [
      { label: "Cancel", value: "cancel" },
      { label: "Without audio", value: "without" },
      { label: "With audio", value: "with", primary: true },
    ],
  });
  if (!answer || answer.choice === "cancel") return;
  const dest = await save({ title: "Export project", defaultPath: `${p.name}.comilito`, filters: COMILITO_FILTER });
  if (dest) await run(api.exportProject(dest, answer.choice === "with"));
}

export async function importProject() {
  const path = await open({ title: "Import project", filters: COMILITO_FILTER });
  if (typeof path === "string") await run(api.importProject(path));
}

/** A backup is imported like any export, as a new project (the current one is untouched). */
export async function restoreBackup() {
  const folder = await run(api.backupsFolder());
  const path = await open({ title: "Restore a backup (opens as a copy)", defaultPath: folder, filters: COMILITO_FILTER });
  if (typeof path === "string") await run(api.importProject(path));
}

export async function relinkFromFolder() {
  const folder = await open({ title: "Folder to search for the missing audio", directory: true });
  if (typeof folder === "string") await run(api.relinkFromFolder(folder));
}

export async function relinkOne(soundId: string) {
  const name = soundById(soundId)?.name ?? "sound";
  const path = await open({ title: `Locate the audio for “${name}”`, filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }] });
  if (typeof path === "string") await run(api.relinkSound(soundId, path));
}

// ---- Previews ---------------------------------------------------------------------------------

/**
 * Stops the preview when the editor it was started in closes, so it doesn't keep playing
 * unseen (TODO.md). With `soundId`, only if that sound is the one previewing: the Library's
 * sound editor is recreated whenever the selection changes, and a preview started from another
 * row should survive that. The mixer bar can stop any preview too.
 */
export function stopPreviewOnLeave(soundId?: string) {
  const playing = ui.snapshot?.playback.preview;
  if (playing && (soundId === undefined || playing === soundId)) run(api.stopPreview());
}
