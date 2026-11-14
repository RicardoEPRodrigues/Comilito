// TypeScript versions of the Rust structs the UI receives (src-tauri/src/project.rs,
// mix.rs, audio.rs, state.rs, prefs.rs, themes.rs, downloads.rs, tools.rs, waveform.rs).
// Field names are camelCase because serde renames them.
// If you change a Rust struct, change it here too.

export type Category = "music" | "ambience" | "effects";
export type Role = "foreground" | "background" | "supporting";
export type Transport = "stopped" | "playing" | "paused";
export type StackStatus = "empty" | "saved" | "modified" | "unsaved";

export const CATEGORIES: { id: Category; label: string }[] = [
  { id: "music", label: "Music" },
  { id: "ambience", label: "Ambience" },
  { id: "effects", label: "Effects" },
];

export const ROLES: Role[] = ["foreground", "background", "supporting"];

export interface PerCategory<T> {
  music: T;
  ambience: T;
  effects: T;
}

export interface FileInfo {
  path: string;
  size: number;
  modified: string;
  sha256?: string;
  loudness?: Loudness;
}

export interface Loudness {
  /** Integrated loudness, LUFS. */
  lufs: number;
  /** Highest sample, linear (1 = full scale). */
  peak: number;
  /** Length of the file, measured with the loudness. */
  durationMs?: number;
}

export interface Normalization {
  music: boolean;
  ambience: boolean;
  effects: boolean;
  targetLufs: number;
}

export interface TimeRange {
  startMs: number;
  endMs: number;
}

export interface Source {
  url: string;
  title?: string;
  site?: string;
  downloadedAt: string;
  range?: TimeRange;
}

export interface Sound {
  id: string;
  category: Category;
  name: string;
  file: FileInfo;
  // Optional fields are overrides; missing means "use the project default".
  volume?: number;
  loop?: boolean;
  fadeInMs?: number;
  fadeOutMs?: number;
  tags?: string[];
  favourite?: boolean;
  source?: Source;
  lastUsed?: string;
}

export interface Playlist {
  tracks: string[];
  shuffle?: boolean;
}

export interface Layer {
  sound: string;
  volume?: number;
  role?: Role;
}

export interface Scene {
  id: string;
  name: string;
  group?: string;
  hotkey?: string;
  music: Playlist | null;
  /** For a scene without music: silence the music of the scenes below. */
  blocksMusic?: boolean;
  ambience: Layer[];
  effects: string[];
  duckLevel?: number;
  transitionMs?: number;
  /** Category volumes while this is the top-most enabled scene; missing: the project's. */
  volumes?: PerCategory<number>;
}

export interface StackEntry {
  id: string;
  scene: string;
  enabled: boolean;
}

export interface PresetEntry {
  scene: string;
  enabled: boolean;
}

export interface StackPreset {
  id: string;
  name: string;
  group?: string;
  stack: PresetEntry[];
}

export interface SoundDefaults {
  loop: boolean;
  fadeInMs: number;
  fadeOutMs: number;
  shuffle?: boolean;
}

export interface ProjectDefaults {
  duckLevel: number;
  transitionMs: number;
  soundVolume: number;
  categories: PerCategory<SoundDefaults>;
  normalize: Normalization;
}

export interface BackupSettings {
  intervalMinutes: number;
  keep: number;
  includeAudio: boolean;
}

export interface Project {
  formatVersion: number;
  name: string;
  masterVolume: number;
  categories: PerCategory<{ volume: number }>;
  sounds: Sound[];
  scenes: Scene[];
  /** Bottom to top. */
  stack: StackEntry[];
  stackPresets: StackPreset[];
  presetGroups?: string[];
  stackOrigin?: string;
  defaults: ProjectDefaults;
  backups: BackupSettings;
}

export interface TargetMix {
  musicEntry: string | null;
  ambience: Record<string, number>;
  effects: string[];
  layerVolumes: Record<string, number[]>;
  /** The scene whose category volumes play (the top-most enabled one), if any. */
  volumesScene: string | null;
  volumes: PerCategory<number>;
}

export interface PlaybackInfo {
  transport: Transport;
  musicSound: string | null;
  playingEffects: string[];
  preview: string | null;
}

export interface Snapshot {
  project: Project | null;
  projectDir: string | null;
  missing: string[];
  mix: TargetMix;
  playback: PlaybackInfo;
  changedOnDisk: boolean;
  hashingRemaining: number;
  analyzingRemaining: number;
  folders: PerCategory<string[]>;
  stackStatus: StackStatus;
  presetGroups: string[];
}

export interface ProjectSummary {
  name: string;
  path: string;
  /** Null when the project file can't be read. */
  scenes: number | null;
  presets: number | null;
  sounds: number | null;
  /** When comilito.json last changed (ISO 8601). */
  modified: string | null;
}

export interface Appearance {
  mode: "system" | "fixed";
  theme: string;
  lightTheme: string;
  darkTheme: string;
  accent: "theme" | "desktop" | "custom";
  customAccent: string;
  capitals: boolean;
  watermark: boolean;
  rules: boolean;
  blackletter: boolean;
}

export interface Preferences {
  formatVersion: number;
  projectsFolder: string | null;
  openLastProject: boolean;
  /** Most recent first. Kept by Rust; changes sent from here are ignored. */
  recentProjects: string[];
  playback: { pauseFadeMs: number; stopFadeMs: number };
  behaviour: {
    duplicateInStack: "ask" | "move" | "duplicate";
    removeSound: "ask" | "trash";
    unsavedChanges: "ask" | "save" | "discard";
    replaceUnsavedTapestry: "ask" | "replace";
  };
  appearance: Appearance;
  newProjectDefaults: {
    masterVolume: number;
    categories: PerCategory<{ volume: number }>;
    defaults: ProjectDefaults;
    backups: BackupSettings;
  };
}

/** A colour theme (built-in ones live in themes.ts; user ones are saved by Rust). */
export interface Theme {
  id: string;
  name: string;
  dark: boolean;
  colors: Record<string, string>;
  fonts: { display: string; body: string };
  /** Files a user theme brought with it (decision 0025), by their stored name
   *  (`<sha256>.<ext>`); read them with `api.readThemeAsset`. */
  watermark?: string;
  /** Font family → font file, for fonts that come with the theme. */
  fontFiles?: Record<string, string>;
  licences?: string[];
}

export interface DesktopAppearance {
  accent: string | null;
  dark: boolean | null;
}

export interface ToolsStatus {
  ytDlp: string | null;
  ffmpeg: boolean;
  size: number;
}

export interface InstallProgress {
  step: string;
  done: number;
  total: number;
}

export interface Probe {
  kind: "direct" | "page";
  url: string;
  title: string;
  site: string;
  durationMs: number | null;
  thumbnail: string | null;
  size: number | null;
  quality: string | null;
  ext: string;
}

export interface DownloadRequest {
  url: string;
  kind: string;
  title: string;
  site: string;
  ext: string;
  category: Category;
  folder: string;
  name: string;
  range: TimeRange | null;
  /** "Download again" into this existing sound. */
  replace?: string | null;
}

export interface DownloadJob {
  id: string;
  name: string;
  url: string;
  site: string;
  status: "queued" | "running" | "done" | "failed" | "cancelled";
  progress: number;
  message: string | null;
  soundId: string | null;
}

export interface WaveformView {
  durationMs: number;
  startMs: number;
  endMs: number;
  peaks: number[];
}

export interface CutOptions {
  startMs: number;
  endMs: number;
  fadeEdges: boolean;
  exact: boolean;
}
