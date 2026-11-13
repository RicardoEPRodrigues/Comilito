// TypeScript versions of the Rust structs the UI receives (src-tauri/src/project.rs,
// mix.rs, audio.rs, state.rs). Field names are camelCase because serde renames them.
// If you change a Rust struct, change it here too.

export type Category = "music" | "ambience" | "effects";
export type Role = "foreground" | "background" | "supporting";
export type Transport = "stopped" | "playing" | "paused";

export const CATEGORIES: { id: Category; label: string; icon: string }[] = [
  { id: "music", label: "Music", icon: "🎵" },
  { id: "ambience", label: "Ambience", icon: "🌲" },
  { id: "effects", label: "Effects", icon: "💥" },
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
  ambience: Layer[];
  effects: string[];
  duckLevel?: number;
  transitionMs?: number;
}

export interface StackEntry {
  id: string;
  scene: string;
  enabled: boolean;
}

export interface StackPreset {
  id: string;
  name: string;
  stack: { scene: string; enabled: boolean }[];
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
  defaults: ProjectDefaults;
  backups: BackupSettings;
}

export interface TargetMix {
  musicEntry: string | null;
  ambience: Record<string, number>;
  effects: string[];
  layerVolumes: Record<string, number[]>;
}

export interface PlaybackInfo {
  transport: Transport;
  musicSound: string | null;
  loopingEffects: string[];
  preview: string | null;
}

export interface Snapshot {
  project: Project | null;
  projectDir: string | null;
  missing: string[];
  mix: TargetMix;
  playback: PlaybackInfo;
  changedOnDisk: boolean;
}

export interface ProjectSummary {
  name: string;
  path: string;
}

export interface Preferences {
  formatVersion: number;
  projectsFolder: string | null;
  playback: { pauseFadeMs: number; stopFadeMs: number };
  behaviour: { duplicateInStack: "ask" | "move" | "duplicate"; removeSound: "ask" | "trash" };
  appearance: { theme: "system" | "light" | "dark" };
  newProjectDefaults: {
    masterVolume: number;
    categories: PerCategory<{ volume: number }>;
    defaults: ProjectDefaults;
    backups: BackupSettings;
  };
}
