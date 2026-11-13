// App-wide UI state, shared by every component.
//
// The `.svelte.ts` extension lets this plain TypeScript module use Svelte's `$state`, so any
// component that reads `ui.snapshot` re-renders when it changes, without passing it down
// through props.
//
// Rust is the source of truth for the project and playback: `ui.snapshot` is replaced
// whole with every `state-changed` event. Everything else here is purely UI state
// (which dialog is open, Play/Edit mode...).

import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { Preferences, Project, Scene, Snapshot, Sound } from "./types";

export type Editing =
  | { kind: "scene"; id: string }
  | { kind: "sound"; id: string }
  | { kind: "projectSettings" }
  | { kind: "preferences" }
  | { kind: "about" }
  | null;

/** What's being dragged right now. Kept here rather than in the browser's dataTransfer,
 *  which is awkward to read during a drag in some webviews. */
export type Drag =
  | { kind: "scene"; id: string }
  | { kind: "preset"; id: string }
  | { kind: "entry"; id: string }
  | null;

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

export const ui = $state({
  snapshot: null as Snapshot | null,
  prefs: null as Preferences | null,
  mode: "play" as "play" | "edit",
  editing: null as Editing,
  drag: null as Drag,
  question: null as Question | null,
  notices: [] as Notice[],
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
    return undefined;
  }
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

export async function init() {
  // Subscribe before fetching, so no event can slip between the two.
  await listen<Snapshot>("state-changed", (e) => {
    ui.snapshot = e.payload;
  });
  await listen<string>("notice", (e) => notify(e.payload));
  ui.snapshot = await api.getState();
  ui.prefs = await api.getPreferences();
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

export function percent(v: number): string {
  return `${Math.round(v * 100)}%`;
}
