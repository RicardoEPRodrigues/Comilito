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
  CutOptions,
  DesktopAppearance,
  DownloadJob,
  DownloadRequest,
  OutputDevice,
  Preferences,
  PresetEntry,
  Probe,
  ProjectDefaults,
  ProjectSummary,
  Role,
  Scene,
  Snapshot,
  Sound,
  Theme,
  ToolsStatus,
  WaveformView,
} from "./types";

export const api = {
  appVersion: () => invoke<string>("app_version"),
  confirmQuit: () => invoke<void>("confirm_quit"),
  getState: () => invoke<Snapshot>("get_state"),
  getPreferences: () => invoke<Preferences>("get_preferences"),
  setPreferences: (prefs: Preferences) => invoke<void>("set_preferences", { prefs }),
  listOutputDevices: () => invoke<{ devices: OutputDevice[]; defaultId: string | null }>("list_output_devices"),

  // Appearance
  desktopAppearance: () => invoke<DesktopAppearance>("desktop_appearance"),
  listUserThemes: () => invoke<Theme[]>("list_user_themes"),
  saveUserTheme: (theme: Theme) => invoke<void>("save_user_theme", { theme }),
  deleteUserTheme: (id: string) => invoke<void>("delete_user_theme", { id }),
  importTheme: (path: string) => invoke<Theme>("import_theme", { path }),
  exportTheme: (theme: Theme, dest: string) => invoke<void>("export_theme", { theme, dest }),
  readThemeAsset: (name: string) => invoke<ArrayBuffer>("read_theme_asset", { name }),
  addThemeFile: (path: string, role: "watermark" | "font" | "licence") => invoke<string>("add_theme_file", { path, role }),

  // Projects
  listProjects: () => invoke<ProjectSummary[]>("list_projects"),
  projectsFolder: () => invoke<string>("projects_folder"),
  duplicateProject: (path: string) => invoke<string>("duplicate_project", { path }),
  /** Checks and starts moving every project and the backups. Returns once the move has
   *  started; use `moveProjects` in store.svelte.ts to wait for the end. */
  moveProjectsFolder: (to: string) => invoke<void>("move_projects_folder", { to }),
  /** The projects folder, or with `project`, that project selected in the file manager. */
  showProjectsFolder: (project: string | null = null) => invoke<void>("show_projects_folder", { project }),
  createProject: (name: string, sample: boolean) => invoke<void>("create_project", { name, sample }),
  openProject: (path: string) => invoke<void>("open_project", { path }),
  closeProject: () => invoke<void>("close_project"),
  reloadProject: () => invoke<void>("reload_project"),
  deleteProject: (path: string, deleteAudio: boolean) =>
    invoke<void>("delete_project", { path, deleteAudio }),
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
  addSounds: (category: Category, paths: string[], folder: string | null = null) =>
    invoke<string[]>("add_sounds", { category, folder, paths }),
  /** A file dropped from the file manager: its bytes go as the raw request body (no JSON,
   *  so large files aren't turned into huge number arrays); the rest goes in headers, which
   *  must be plain ASCII, hence the URL-encoding. Returns the new sound's id. */
  importDroppedFile: (category: Category, folder: string, name: string, bytes: ArrayBuffer) =>
    invoke<string>("import_dropped_file", new Uint8Array(bytes), {
      headers: {
        "x-name": encodeURIComponent(name),
        "x-category": category,
        "x-folder": encodeURIComponent(folder),
      },
    }),
  updateSounds: (sounds: Sound[]) => invoke<void>("update_sounds", { sounds }),
  removeSound: (id: string, permanently: boolean) =>
    invoke<void>("remove_sound", { id, permanently }),
  previewSound: (id: string) => invoke<void>("preview_sound", { id }),
  previewRange: (id: string, startMs: number, endMs: number, seam: boolean) =>
    invoke<void>("preview_range", { id, startMs: Math.round(startMs), endMs: Math.round(endMs), seam }),
  stopPreview: () => invoke<void>("stop_preview"),
  previewPosition: () => invoke<[string, number] | null>("preview_position"),
  logMessage: (level: "info" | "warn" | "error", message: string) => invoke<void>("log_message", { level, message }),
  openLogFile: () => invoke<void>("open_log_file"),
  forgetAllData: (keepBackups: boolean) => invoke<void>("forget_all_data", { keepBackups }),
  toggleEffect: (id: string) => invoke<void>("toggle_effect", { id }),
  moveSounds: (ids: string[], category: Category, folder: string) =>
    invoke<void>("move_sounds", { ids, category, folder }),
  soundWaveform: (id: string, buckets: number, startMs?: number, endMs?: number) =>
    invoke<WaveformView>("sound_waveform", {
      id,
      buckets: Math.max(1, Math.round(buckets)),
      startMs: startMs === undefined ? null : Math.round(startMs),
      endMs: endMs === undefined ? null : Math.round(endMs),
    }),
  cropSound: (id: string, options: CutOptions, overwrite: boolean) =>
    invoke<string>("crop_sound", {
      id,
      options: { ...options, startMs: Math.round(options.startMs), endMs: Math.round(options.endMs) },
      overwrite,
    }),
  revealSound: (id: string) => invoke<void>("reveal_sound", { id }),

  // Library folders
  createFolder: (category: Category, folder: string) =>
    invoke<void>("create_folder", { category, folder }),
  renameFolder: (category: Category, from: string, to: string) =>
    invoke<void>("rename_folder", { category, from, to }),
  deleteFolder: (category: Category, folder: string) =>
    invoke<void>("delete_folder", { category, folder }),
  openAudioFolder: (category: Category | null, folder: string | null) =>
    invoke<void>("open_audio_folder", { category, folder }),

  // Scenes
  createScene: (name: string, group: string | null) =>
    invoke<string>("create_scene", { name, group }),
  updateScene: (scene: Scene) => invoke<void>("update_scene", { scene }),
  duplicateScene: (id: string) => invoke<string>("duplicate_scene", { id }),
  deleteScene: (id: string) => invoke<void>("delete_scene", { id }),
  sceneGroupRename: (from: string, to: string) => invoke<void>("scene_group_rename", { from, to }),
  sceneSetGroup: (sceneId: string, group: string | null) => invoke<void>("scene_set_group", { sceneId, group }),
  sceneGroupUngroup: (name: string) => invoke<void>("scene_group_ungroup", { name }),
  addSoundToScene: (sceneId: string, soundId: string) =>
    invoke<void>("add_sound_to_scene", { sceneId, soundId }),
  removeSoundFromScene: (sceneId: string, soundId: string) =>
    invoke<void>("remove_sound_from_scene", { sceneId, soundId }),
  removeMissingSounds: () => invoke<void>("remove_missing_sounds"),
  setLayerVolume: (sceneId: string, layerIndex: number, volume: number) =>
    invoke<void>("set_layer_volume", { sceneId, layerIndex, volume }),
  setLayerRole: (sceneId: string, layerIndex: number, role: Role) =>
    invoke<void>("set_layer_role", { sceneId, layerIndex, role }),

  // Stack (indexes count from the bottom: 0 = bottom)
  stackInsert: (sceneId: string, index: number, mode: "move" | "duplicate") =>
    invoke<void>("stack_insert", { sceneId, index, mode }),
  stackMove: (entryId: string, index: number) => invoke<void>("stack_move", { entryId, index }),
  stackRemove: (entryId: string) => invoke<void>("stack_remove", { entryId }),
  stackSetEnabled: (entryId: string, enabled: boolean) =>
    invoke<void>("stack_set_enabled", { entryId, enabled }),
  stackSwitch: (sceneId: string) => invoke<void>("stack_switch", { sceneId }),
  stackClear: () => invoke<void>("stack_clear"),
  sceneHotkey: (sceneId: string) => invoke<void>("scene_hotkey", { sceneId }),

  // Presets
  presetSave: (name: string, group: string | null) => invoke<string>("preset_save", { name, group }),
  presetUpdate: (presetId: string) => invoke<void>("preset_update", { presetId }),
  presetSetEntries: (presetId: string, entries: PresetEntry[]) =>
    invoke<void>("preset_set_entries", { presetId, entries }),
  presetLoad: (presetId: string) => invoke<void>("preset_load", { presetId }),
  presetInsert: (presetId: string, index: number) =>
    invoke<void>("preset_insert", { presetId, index }),
  presetRename: (presetId: string, name: string) =>
    invoke<void>("preset_rename", { presetId, name }),
  presetDuplicate: (presetId: string) => invoke<string>("preset_duplicate", { presetId }),
  presetSetGroup: (presetId: string, group: string | null) =>
    invoke<void>("preset_set_group", { presetId, group }),
  presetDelete: (presetId: string) => invoke<void>("preset_delete", { presetId }),
  presetGroupCreate: (name: string) => invoke<void>("preset_group_create", { name }),
  presetGroupRename: (from: string, to: string) => invoke<void>("preset_group_rename", { from, to }),
  presetGroupDelete: (name: string) => invoke<void>("preset_group_delete", { name }),

  // Tools and downloads
  toolsStatus: () => invoke<ToolsStatus>("tools_status"),
  installTools: () => invoke<ToolsStatus>("install_tools"),
  toolsUpdateAvailable: () => invoke<boolean | null>("tools_update_available"),
  removeTools: () => invoke<void>("remove_tools"),
  probeUrl: (url: string) => invoke<Probe>("probe_url", { url }),
  startDownload: (request: DownloadRequest) => invoke<string>("start_download", { request }),
  cancelDownload: (id: string) => invoke<void>("cancel_download", { id }),
  clearDownloads: () => invoke<void>("clear_downloads"),
  listDownloads: () => invoke<DownloadJob[]>("list_downloads"),

  // Transport
  togglePlay: () => invoke<void>("toggle_play"),
  stopAll: () => invoke<void>("stop_all"),
};
