// Finding sounds in the Library (decisions 0017 and 0018): smart lists, categories, folders,
// and search by name or #tag. Shared by the Library drawer and the Library workspace.

import type { Category, Project, Sound } from "./types";
import { folderOf } from "./store.svelte";

export type Filter = "all" | "favourites" | "recent" | "unused" | "missing" | "tapestry";

export interface Query {
  filter: Filter;
  category?: Category | null;
  /** null = any folder; "" = the category folder itself (not subfolders). */
  folder?: string | null;
  /** Include sounds in subfolders of `folder`. */
  deep?: boolean;
  search?: string;
}

/** Sound ids used by any scene. */
export function usedSoundIds(project: Project): Set<string> {
  const used = new Set<string>();
  for (const s of project.scenes) {
    s.music?.tracks.forEach((t) => used.add(t));
    s.ambience.forEach((l) => used.add(l.sound));
    s.effects.forEach((e) => used.add(e));
  }
  return used;
}

/** Sound ids used by the enabled scenes in the tapestry. */
export function tapestrySoundIds(project: Project): Set<string> {
  const scenes = new Set(project.stack.filter((e) => e.enabled).map((e) => e.scene));
  const used = new Set<string>();
  for (const s of project.scenes) {
    if (!scenes.has(s.id)) continue;
    s.music?.tracks.forEach((t) => used.add(t));
    s.ambience.forEach((l) => used.add(l.sound));
    s.effects.forEach((e) => used.add(e));
  }
  return used;
}

const RECENT_DAYS = 30;

/** Does a sound match the search text? Words must all match; `#word` matches a tag. */
export function matchesSearch(sound: Sound, search: string): boolean {
  const words = search.toLowerCase().split(/\s+/).filter(Boolean);
  const name = sound.name.toLowerCase();
  return words.every((w) =>
    w.startsWith("#")
      ? (sound.tags ?? []).some((t) => t.startsWith(w.slice(1)))
      : name.includes(w) || (sound.tags ?? []).some((t) => t === w),
  );
}

export function findSounds(project: Project, missing: string[], q: Query): Sound[] {
  const used = q.filter === "unused" ? usedSoundIds(project) : null;
  const inTapestry = q.filter === "tapestry" ? tapestrySoundIds(project) : null;
  const missingSet = new Set(missing);
  const cutoff = Date.now() - RECENT_DAYS * 24 * 3600 * 1000;

  let list = project.sounds.filter((s) => {
    if (q.category && s.category !== q.category) return false;
    if (q.folder !== null && q.folder !== undefined) {
      const f = folderOf(s);
      if (q.deep ? !(f === q.folder || f.startsWith(`${q.folder}/`) || q.folder === "") : f !== q.folder) return false;
    }
    switch (q.filter) {
      case "favourites":
        if (!s.favourite) return false;
        break;
      case "recent":
        if (!s.lastUsed || Date.parse(s.lastUsed) < cutoff) return false;
        break;
      case "unused":
        if (used!.has(s.id)) return false;
        break;
      case "missing":
        if (!missingSet.has(s.id)) return false;
        break;
      case "tapestry":
        if (!inTapestry!.has(s.id)) return false;
        break;
    }
    return !q.search || matchesSearch(s, q.search);
  });

  if (q.filter === "recent") list = list.sort((a, b) => (b.lastUsed ?? "").localeCompare(a.lastUsed ?? ""));
  else list = list.sort((a, b) => a.name.localeCompare(b.name));
  return list;
}

/** The most used tags in the project, for quick filter chips. */
export function topTags(project: Project, limit: number): string[] {
  const counts = new Map<string, number>();
  for (const s of project.sounds) for (const t of s.tags ?? []) counts.set(t, (counts.get(t) ?? 0) + 1);
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, limit)
    .map(([t]) => t);
}

/** Starts dragging a sound (onto a scene, a tapestry entry or the scene editor). */
export function soundDragStart(e: DragEvent, id: string, setDrag: (id: string) => void) {
  setDrag(id);
  e.dataTransfer?.setData("text/plain", id);
  // "copyMove", not "copy": WebKit defaults a drop to "move" and silently refuses it otherwise.
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "copyMove";
}
