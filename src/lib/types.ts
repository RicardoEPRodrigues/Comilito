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
export const roleLabel = (r: Role) => r[0].toUpperCase() + r.slice(1);

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
  /** Loudness normalization; missing follows the project's switch for the category. */
  normalize?: boolean;
  tags?: string[];
  favourite?: boolean;
  source?: Source;
  lastUsed?: string;
}

/** A video file in the library (decision 0035). Its sound is never played. */
/** Video on this computer (Preferences › Video). */
export interface VideoPrefs {
  enabled: boolean;
  /** Screen name, or null for the main screen. */
  screen: string | null;
  fullscreen: boolean;
  fit: "cover" | "contain";
  crossfadeMs: number;
  /** 720, 1080, 1440, or 0 for the best available. */
  downloadHeight: number;
  convert: "ask" | "always" | "never";
  /** Space pauses the sound only; the video keeps playing. Esc still blanks it. */
  keepPlayingWhenPaused: boolean;
}

export interface Video {
  id: string;
  name: string;
  /** Missing in files from before kinds existed: ambience. */
  kind?: VideoKind;
  /** `path` is under `video/`. */
  file: FileInfo;
  /** Measured in the background with ffprobe; missing until then. */
  info?: VideoInfo;
  tags?: string[];
  favourite?: boolean;
  source?: Source;
  lastUsed?: string;
}

export interface VideoInfo {
  container: string;
  codec: string;
  width: number;
  height: number;
  durationMs: number;
  /** WebM with VP8, VP9 or AV1: plays in every webview. Others are offered for conversion. */
  playsEverywhere: boolean;
}

/** A scene's videos. The settings are overrides; missing follows `defaults.video`. */
export interface VideoPlaylist {
  videos: string[];
  shuffle?: boolean;
  /** Minutes before switching to the next video; 0 plays each to its end. */
  switchMinutes?: number;
  /** Coming back to a video: continue where it stopped, or start over. */
  resume?: boolean;
}

export interface VideoDefaults {
  shuffle: boolean;
  switchMinutes: number;
  resume: boolean;
}

export interface Playlist {
  tracks: string[];
  shuffle?: boolean;
}

export interface Layer {
  sound: string;
  volume?: number;
  /** Always set; older files without one load as "foreground" (decision 0029). */
  role: Role;
}

export interface Scene {
  id: string;
  name: string;
  group?: string;
  hotkey?: string;
  music: Playlist | null;
  /** For a scene without music: silence the music of the scenes below. */
  blocksMusic?: boolean;
  /** Duck all the ambience below, whatever its role (decision 0031). */
  ducksAll?: boolean;
  ambience: Layer[];
  effects: string[];
  duckLevel?: number;
  transitionMs?: number;
  /** Category volumes while this is the top-most enabled scene; missing: the project's. */
  volumes?: PerCategory<number>;
  /** Missing: the video of the scenes below keeps showing (unless `hidesVideo`). */
  video?: VideoPlaylist;
  /** For a scene without videos: hide the video below (a black screen). */
  hidesVideo?: boolean;
  /** Video ids on the video effects pad. */
  videoEffects?: string[];
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
  /** Missing in projects from before videos (decision 0035). */
  video?: VideoDefaults;
}

export interface BackupSettings {
  intervalMinutes: number;
  keep: number;
  includeAudio: boolean;
  includeVideo?: boolean;
}

export interface Project {
  formatVersion: number;
  name: string;
  masterVolume: number;
  categories: PerCategory<{ volume: number }>;
  sounds: Sound[];
  /** Missing in projects without videos. */
  videos?: Video[];
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
  /** What the video window shows: the top-most enabled scene with videos (null: black). */
  video: VideoTarget | null;
  /** Video effects of the enabled scenes, top scene first. */
  videoEffects: string[];
}

/** A scene's playlist with the inherited settings filled in (decision 0035). */
export interface VideoTarget {
  entryId: string;
  sceneId: string;
  videos: string[];
  shuffle: boolean;
  switchMinutes: number;
  resume: boolean;
}

/** Sent by the video window about once a second, for the bottom bar. */
export interface VideoStatus {
  videoId: string | null;
  sceneId: string | null;
  positionMs: number;
  /** Until the next video, when switching is on and there's more than one. */
  switchInMs: number | null;
  /** The video effect playing over the playlist, if any. */
  effect: string | null;
}

export interface Screen {
  name: string;
  width: number;
  height: number;
  primary: boolean;
}

export interface PlaybackInfo {
  transport: Transport;
  musicSound: string | null;
  playingEffects: string[];
  preview: string | null;
}

/** An audio output to choose from (Preferences › Playback). */
export interface OutputDevice {
  id: string;
  name: string;
}

/** A part of the Library with its own folder: an audio category, or a kind of video. */
export type Shelf = Category | "ambienceVideo" | "videoEffect";

/** What a video is for (decision 0039): ambience videos play in scenes' playlists, video
 *  effects on the video effects pad. */
export type VideoKind = "ambience" | "effect";

export interface Snapshot {
  /** How long the category volumes take to reach `mix.volumes`, for the mixer's sliders. */
  levelsFadeMs: number;
  project: Project | null;
  projectDir: string | null;
  missing: string[];
  mix: TargetMix;
  playback: PlaybackInfo;
  changedOnDisk: boolean;
  hashingRemaining: number;
  analyzingRemaining: number;
  folders: PerCategory<string[]>;
  /** Subfolders of `video/ambience/` and `video/effects/`. */
  videoFolders: { ambience: string[]; effect: string[] };
  videoWindowOpen: boolean;
  /** The video effect playing in the video window. */
  videoEffect: string | null;
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
  /** `outputDevice`: an `OutputDevice` id, or null for the system default. */
  playback: { outputDevice: string | null; pauseFadeMs: number; stopFadeMs: number };
  behaviour: {
    duplicateInStack: "ask" | "move" | "duplicate";
    removeSound: "ask" | "trash";
    unsavedChanges: "ask" | "save" | "discard";
    replaceUnsavedTapestry: "ask" | "replace";
    /** Ask before quitting while downloads, fingerprinting, a backup or a move are running. */
    warnOnQuit: boolean;
  };
  /** `KeyboardEvent.key` values; "" means the shortcut is off. */
  shortcuts: { removeTopScene: string };
  appearance: Appearance;
  video: VideoPrefs;
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
  /** A direct link to a video file (decision 0035). */
  isVideo: boolean;
  /** For pages: the picture alone at each resolution limit. */
  videoOptions: VideoOption[];
}

export interface VideoOption {
  /** 720, 1080, 1440, or 0 for the best. */
  maxHeight: number;
  height: number;
  codec: string;
  ext: string;
  size: number | null;
  playsEverywhere: boolean;
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
  /** "Download again" into this existing sound (or video). */
  replace?: string | null;
  /** Download the picture only, as a video, up to this height (0: the best); null: audio. */
  videoHeight?: number | null;
  /** For a video: ambience video or video effect. */
  videoKind?: VideoKind;
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

/** A video being converted to WebM (decision 0035). `id` is the video's id. */
export interface ConversionJob {
  id: string;
  name: string;
  status: "queued" | "running" | "done" | "failed" | "cancelled";
  progress: number;
  message: string | null;
}

/** Videos that may not play, offered for conversion. "library": asked from a video's page. */
export interface ConvertOffer {
  ids: string[];
  origin: "added" | "folder" | "library";
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
