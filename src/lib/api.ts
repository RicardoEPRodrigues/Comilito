// One typed function per Rust command (src-tauri/src/commands.rs), so the rest of the UI
// never types a command name as a string. Argument names must match the Rust parameter
// names; Tauri converts camelCase here to snake_case there (sceneId -> scene_id).
//
// Commands that change something don't return the new state: it arrives through the
// `state-changed` event (see store.svelte.ts).

import { invoke } from "@tauri-apps/api/core";
import type {
  BackupSettings,
  Category,
  Preferences,
  ProjectDefaults,
  ProjectSummary,
  Scene,
  Snapshot,
  Sound,
} from "./types";

export const api = {
  appVersion: () => invoke<string>("app_version"),
  getState: () => invoke<Snapshot>("get_state"),
  getPreferences: () => invoke<Preferences>("get_preferences"),
  setPreferences: (prefs: Preferences) => invoke<void>("set_preferences", { prefs }),

  // Projects
  listProjects: () => invoke<ProjectSummary[]>("list_projects"),
  projectsFolder: () => invoke<string>("projects_folder"),
  createProject: (name: string, sample: boolean) => invoke<void>("create_project", { name, sample }),
  openProject: (path: string) => invoke<void>("open_project", { path }),
  closeProject: () => invoke<void>("close_project"),
  reloadProject: () => invoke<void>("reload_project"),
  keepMyVersion: () => invoke<void>("keep_my_version"),
  renameProject: (name: string) => invoke<void>("rename_project", { name }),
  exportProject: (dest: string, includeAudio: boolean) =>
    invoke<void>("export_project", { dest, includeAudio }),
  importProject: (path: string) => invoke<void>("import_project", { path }),
  backupsFolder: () => invoke<string>("backups_folder"),
  relinkFromFolder: (folder: string) => invoke<void>("relink_from_folder", { folder }),
  relinkSound: (id: string, path: string) => invoke<void>("relink_sound", { id, path }),

  // Project settings
  setMasterVolume: (volume: number) => invoke<void>("set_master_volume", { volume }),
  setCategoryVolume: (category: Category, volume: number) =>
    invoke<void>("set_category_volume", { category, volume }),
  setProjectDefaults: (defaults: ProjectDefaults) =>
    invoke<void>("set_project_defaults", { defaults }),
  setBackupSettings: (backups: BackupSettings) =>
    invoke<void>("set_backup_settings", { backups }),
  resetProjectDefaults: () => invoke<void>("reset_project_defaults"),

  // Sounds
  addSounds: (category: Category, paths: string[]) =>
    invoke<void>("add_sounds", { category, paths }),
  updateSound: (sound: Sound) => invoke<void>("update_sound", { sound }),
  removeSound: (id: string, permanently: boolean) =>
    invoke<void>("remove_sound", { id, permanently }),
  previewSound: (id: string) => invoke<void>("preview_sound", { id }),
  playEffect: (id: string) => invoke<void>("play_effect", { id }),

  // Scenes
  createScene: (name: string, group: string | null) =>
    invoke<string>("create_scene", { name, group }),
  updateScene: (scene: Scene) => invoke<void>("update_scene", { scene }),
  duplicateScene: (id: string) => invoke<string>("duplicate_scene", { id }),
  deleteScene: (id: string) => invoke<void>("delete_scene", { id }),
  setLayerVolume: (sceneId: string, layerIndex: number, volume: number) =>
    invoke<void>("set_layer_volume", { sceneId, layerIndex, volume }),

  // Stack (indexes count from the bottom: 0 = bottom)
  stackInsert: (sceneId: string, index: number, mode: "move" | "duplicate") =>
    invoke<void>("stack_insert", { sceneId, index, mode }),
  stackMove: (entryId: string, index: number) => invoke<void>("stack_move", { entryId, index }),
  stackRemove: (entryId: string) => invoke<void>("stack_remove", { entryId }),
  stackSetEnabled: (entryId: string, enabled: boolean) =>
    invoke<void>("stack_set_enabled", { entryId, enabled }),
  stackSwitch: (sceneId: string) => invoke<void>("stack_switch", { sceneId }),
  sceneHotkey: (sceneId: string) => invoke<void>("scene_hotkey", { sceneId }),

  // Presets
  presetSave: (name: string) => invoke<void>("preset_save", { name }),
  presetLoad: (presetId: string) => invoke<void>("preset_load", { presetId }),
  presetInsert: (presetId: string, index: number) =>
    invoke<void>("preset_insert", { presetId, index }),
  presetRename: (presetId: string, name: string) =>
    invoke<void>("preset_rename", { presetId, name }),
  presetDelete: (presetId: string) => invoke<void>("preset_delete", { presetId }),

  // Transport
  togglePlay: () => invoke<void>("toggle_play"),
  stopAll: () => invoke<void>("stop_all"),
};
