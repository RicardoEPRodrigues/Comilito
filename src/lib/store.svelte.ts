// App-wide UI state, shared by every component.
//
// The `.svelte.ts` extension lets this plain TypeScript module use Svelte's `$state`, so any
// component that reads `ui.snapshot` re-renders when it changes, without passing it down
// through props.
//
// Rust is the source of truth for the project and playback: `ui.snapshot` is replaced
// whole with every `state-changed` event. Everything else here is purely UI state
// (which view is shown, what's being edited or dragged...).

import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type {
  Appearance,
  Category,
  DesktopAppearance,
  DownloadJob,
  InstallProgress,
  Preferences,
  Project,
  Scene,
  Snapshot,
  Sound,
  StackPreset,
  Theme,
  ToolsStatus,
} from "./types";

/** Windows shown over everything (modals). */
export type Editing =
  | { kind: "projectSettings" }
  | { kind: "preferences"; page?: string }
  | { kind: "about" }
  | { kind: "addUrl"; prefill?: UrlPrefill; target?: { category: Category; folder: string } }
  | null;

/** "Download again": the URL dialog opens with the original download's settings. */
export interface UrlPrefill {
  url: string;
  name: string;
  category: Category;
  folder: string;
  range: { startMs: number; endMs: number } | null;
  /** The sound whose file the new download replaces. */
  replace: string;
}

/**
 * What the centre panel is editing in place (decisions 0017 and 0018), Unity-prefab style.
 * `from` is the preset a scene was opened from, for the breadcrumbs.
 */
export type Focus =
  | { kind: "scene"; id: string; from?: string }
  | { kind: "preset"; id: string }
  | null;

/** What's being dragged right now. Kept here rather than in the browser's dataTransfer,
 *  which is awkward to read during a drag in some webviews. */
export type Drag =
  | { kind: "scene"; id: string }
  | { kind: "preset"; id: string }
  | { kind: "entry"; id: string }
  | { kind: "sound"; id: string }
  | { kind: "folder"; category: Category; folder: string }
  | { kind: "presetEntry"; index: number }
  | null;

export type SmartList = "all" | "favourites" | "recent" | "unused" | "missing";

/** A question with buttons (and optionally a text field); see Dialog.svelte. */
export interface Question {
  title: string;
  message?: string;
  input?: { label: string; value: string };
  choices: { label: string; value: string; primary?: boolean; danger?: boolean }[];
  resolve: (answer: { choice: string; text: string } | null) => void;
}

export interface Notice {
  id: number;
  message: string;
}

function remembered<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(`comilito.${key}`);
    return v === null ? fallback : (JSON.parse(v) as T);
  } catch {
    return fallback;
  }
}

/** Remembers a small UI choice on this computer (drawer open, active tab...). */
export function remember(key: string, value: unknown) {
  try {
    localStorage.setItem(`comilito.${key}`, JSON.stringify(value));
  } catch {
    // Not remembered this time; nothing depends on it.
  }
}

export const ui = $state({
  snapshot: null as Snapshot | null,
  prefs: null as Preferences | null,
  editing: null as Editing,
  focus: null as Focus,
  view: "tapestry" as "tapestry" | "library",
  leftTab: remembered<"scenes" | "presets">("leftTab", "scenes"),
  drawerOpen: remembered<boolean>("drawerOpen", true),
  /** Height of the open Library drawer in px, once resized; null = the default size. */
  drawerHeight: remembered<number | null>("drawerHeight", null),
  /** Scene / preset group headings the user folded away. */
  collapsed: remembered<string[]>("collapsed", []),
  library: {
    smart: "all" as SmartList,
    category: null as Category | null,
    /** null = the whole category; "" = the category folder itself. */
    folder: null as string | null,
    search: "",
    selected: [] as string[],
    cropping: false,
  },
  drag: null as Drag,
  /** While files from outside are dragged over a Library folder or category: which one, as
   *  `category:folder` ("" for the category folder). See FileDropImport.svelte. */
  fileDropTarget: null as string | null,
  /** While the projects folder is being moved (a dialog covers the app). */
  moving: null as null | { doneBytes: number; totalBytes: number; current: string },
  question: null as Question | null,
  notices: [] as Notice[],
  desktop: null as DesktopAppearance | null,
  userThemes: [] as Theme[],
  /** The active theme's own watermark as a blob: URL, or null for the skeletal horse. */
  themeWatermark: null as string | null,
  downloads: [] as DownloadJob[],
  tools: null as ToolsStatus | null,
  toolsProgress: null as InstallProgress | null,
  /** Registered by an in-place editor with unsaved changes; see `leaveFocus`. */
  guard: null as null | (() => Promise<boolean>),
  /** While Preferences is open: the appearance / theme being edited, shown live. */
  previewAppearance: null as null | Appearance,
  previewTheme: null as null | Theme,
  /** Save / Discard of the open in-place editor, shown in the context bar. */
  editorActions: null as null | { dirty: boolean; save: () => void; discard: () => void; saveLabel: string },
});

let nextNoticeId = 1;

export function notify(message: string) {
  const id = nextNoticeId++;
  ui.notices.push({ id, message });
  setTimeout(() => {
    ui.notices = ui.notices.filter((n) => n.id !== id);
  }, 6000);
}

/** Runs a command and shows its error, if any, as a notice instead of throwing. */
export async function run<T>(promise: Promise<T>): Promise<T | undefined> {
  try {
    return await promise;
  } catch (err) {
    notify(String(err));
    // Keep a record in the log file ("See logs"). Ignore failures: the log is a bonus.
    api.logMessage("error", String(err)).catch(() => {});
    return undefined;
  }
}

/** "See logs": opens the log file. */
export function openLogs() {
  return run(api.openLogFile());
}

/** Shows a question and waits for the answer (`null` if cancelled). */
export function ask(q: Omit<Question, "resolve">): Promise<{ choice: string; text: string } | null> {
  return new Promise((resolve) => {
    ui.question = { ...q, resolve };
  });
}

/** Asks for a line of text. Returns `null` if cancelled or left empty. */
export async function askText(title: string, label: string, value = ""): Promise<string | null> {
  const answer = await ask({
    title,
    input: { label, value },
    choices: [
      { label: "Cancel", value: "cancel" },
      { label: "OK", value: "ok", primary: true },
    ],
  });
  if (answer?.choice !== "ok") return null;
  const text = answer.text.trim();
  return text === "" ? null : text;
}

/** Asks for confirmation; resolves true if the user chose `confirm`. */
export async function confirmAction(title: string, message: string, confirm: string, danger = false): Promise<boolean> {
  const answer = await ask({
    title,
    message,
    choices: [
      { label: "Cancel", value: "cancel" },
      { label: confirm, value: "ok", primary: true, danger },
    ],
  });
  return answer?.choice === "ok";
}

/**
 * Leaves the current in-place editor. If it has unsaved changes, its guard asks (or saves or
 * discards, per the preferences). Resolves false if the user chose to keep editing.
 */
export async function leaveFocus(): Promise<boolean> {
  if (ui.guard && !(await ui.guard())) return false;
  ui.guard = null;
  ui.editorActions = null;
  ui.focus = null;
  ui.library.cropping = false;
  return true;
}

/** Opens something in the centre panel, after leaving whatever was open. */
export async function focusOn(focus: Focus) {
  if (!(await leaveFocus())) return;
  ui.view = "tapestry";
  ui.focus = focus;
}

/** Switches between the Tapestry and the Library workspace. */
export async function showView(view: "tapestry" | "library") {
  if (!(await leaveFocus())) return;
  ui.view = view;
}

/** Opens a sound in the Library workspace's editor. */
export async function openSoundInLibrary(soundId: string) {
  if (!(await leaveFocus())) return;
  const sound = soundById(soundId);
  ui.view = "library";
  if (sound) {
    ui.library.smart = "all";
    ui.library.category = sound.category;
    ui.library.folder = folderOf(sound);
  }
  ui.library.selected = [soundId];
}

export async function refreshUserThemes() {
  ui.userThemes = (await run(api.listUserThemes())) ?? [];
}

export async function refreshTools() {
  ui.tools = (await run(api.toolsStatus())) ?? null;
}

export async function init() {
  // Subscribe before fetching, so no event can slip between the two.
  await listen<Snapshot>("state-changed", (e) => {
    ui.snapshot = e.payload;
  });
  await listen<string>("notice", (e) => notify(e.payload));
  await listen<DesktopAppearance>("desktop-appearance", (e) => {
    ui.desktop = e.payload;
  });
  await listen<DownloadJob[]>("downloads-changed", (e) => {
    ui.downloads = e.payload;
  });
  await listen<{ doneBytes: number; totalBytes: number; current: string }>("projects-move-progress", (e) => {
    ui.moving = e.payload;
  });
  await listen<InstallProgress>("tools-progress", (e) => {
    ui.toolsProgress = e.payload;
  });
  // Closing the window while work is running: Rust holds the close back and lists the work.
  await listen<string[]>("quit-requested", async (e) => {
    const answer = await ask({
      title: "Quit while busy?",
      message: `Comilito is still working:\n\n${e.payload.map((w) => `• ${w}`).join("\n")}`,
      choices: [
        { label: "Keep running", value: "stay", primary: true },
        { label: "Quit anyway", value: "quit", danger: true },
      ],
    });
    if (answer?.choice === "quit") await run(api.confirmQuit());
  });
  const [snapshot, prefs, desktop, downloads] = await Promise.all([
    api.getState(),
    api.getPreferences(),
    api.desktopAppearance(),
    api.listDownloads(),
  ]);
  ui.snapshot = snapshot;
  ui.prefs = prefs;
  ui.desktop = desktop;
  ui.downloads = downloads;
  await refreshUserThemes();
}

/** Moves every project and the backups to `to` (decision 0023). Rust does it on a background
 *  thread; this resolves with how many folders moved when it ends, or rejects with the reason.
 *  `ui.moving` drives the progress dialog meanwhile. */
export async function moveProjects(to: string): Promise<number> {
  // Listen before starting, so the end can't slip past.
  // In an object, because TypeScript can't see the assignment inside the callback.
  const sub: { stop?: () => void } = {};
  const finished = new Promise<number>((resolve, reject) => {
    listen<{ moved: number | null; error: string | null }>("projects-moved", (e) => {
      sub.stop?.();
      ui.moving = null;
      if (e.payload.moved !== null && !e.payload.error) resolve(e.payload.moved);
      else reject(e.payload.error ?? "the move failed");
    }).then((u) => (sub.stop = u));
  });
  try {
    ui.moving = { doneBytes: 0, totalBytes: 0, current: "" };
    await api.moveProjectsFolder(to);
  } catch (e) {
    // Refused before anything moved (a name clash, for example).
    ui.moving = null;
    sub.stop?.();
    throw e;
  }
  const moved = await finished;
  // Rust saved the new folder and the recent list; keep the local copy in step.
  const prefs = await api.getPreferences();
  if (ui.prefs) ui.prefs = { ...ui.prefs, projectsFolder: prefs.projectsFolder, recentProjects: prefs.recentProjects };
  return moved;
}

/** Saves preferences and keeps the local copy in step. */
export async function savePrefs(prefs: Preferences) {
  ui.prefs = prefs;
  await run(api.setPreferences($state.snapshot(prefs)));
}

// Small lookups used all over the UI.

export function project(): Project | null {
  return ui.snapshot?.project ?? null;
}

export function soundById(id: string): Sound | undefined {
  return project()?.sounds.find((s) => s.id === id);
}

export function sceneById(id: string): Scene | undefined {
  return project()?.scenes.find((s) => s.id === id);
}

export function presetById(id: string): StackPreset | undefined {
  return project()?.stackPresets.find((p) => p.id === id);
}

/** The Library folder of a sound: the path between `audio/<category>/` and the file. */
export function folderOf(sound: Sound): string {
  const parts = sound.file.path.split("/");
  return parts.length <= 3 ? "" : parts.slice(2, -1).join("/");
}

export function percent(v: number): string {
  return `${Math.round(v * 100)}%`;
}

export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

/** A sound's length, if it has been measured (see loudness.rs). */
export function durationOf(sound: Sound): number | null {
  return sound.file.loudness?.durationMs ?? null;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

/** Scenes using a sound, and how. */
export function scenesUsing(soundId: string): Scene[] {
  return (
    project()?.scenes.filter(
      (s) => s.music?.tracks.includes(soundId) || s.ambience.some((l) => l.sound === soundId) || s.effects.includes(soundId),
    ) ?? []
  );
}
