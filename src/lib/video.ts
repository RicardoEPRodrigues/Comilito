// Videos in the Library (decision 0035): finding them, adding and removing files, and the
// URL the webview plays them from. Kept apart from the sound code, like src-tauri/src/video.rs.

import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { ask, notify, project, run, ui } from "./store.svelte";
import type { Scene, Shelf, Video, VideoKind } from "./types";

/** The two video shelves (decision 0039), in the order the Library shows them. */
export const VIDEO_SHELVES: { id: Shelf; kind: VideoKind; label: string; root: string }[] = [
  { id: "ambienceVideo", kind: "ambience", label: "Ambience videos", root: "video/ambience/" },
  { id: "videoEffect", kind: "effect", label: "Video effects", root: "video/effects/" },
];

export const isVideoShelf = (shelf: Shelf | null): boolean => shelf === "ambienceVideo" || shelf === "videoEffect";
export const kindOfShelf = (shelf: Shelf): VideoKind => (shelf === "videoEffect" ? "effect" : "ambience");
export const shelfOfKind = (kind: VideoKind | undefined): Shelf => (kind === "effect" ? "videoEffect" : "ambienceVideo");
export const kindOf = (video: Video): VideoKind => video.kind ?? "ambience";

// Keep in step with VIDEO_EXTENSIONS in video.rs.
export const VIDEO_EXTENSIONS = ["webm", "mp4", "m4v", "mkv", "mov", "avi", "ogv"];

export const isVideoName = (name: string) => VIDEO_EXTENSIONS.includes(name.split(".").pop()?.toLowerCase() ?? "");

/** Whether the video feature is on (Preferences › Video). */
export function videoEnabled(): boolean {
  return ui.prefs?.video.enabled ?? true;
}

export function videoById(id: string): Video | undefined {
  return project()?.videos?.find((v) => v.id === id);
}

/** The Library folder a video is in: the path between `video/<kind>/` and the file name. */
export function folderOfVideo(video: Video): string {
  const parts = video.file.path.split("/");
  return parts.length <= 3 ? "" : parts.slice(2, -1).join("/");
}

// Loading a video into a <video> element is in videoStream.ts (decision 0037).

/** Scenes that show a video, in their playlist or as a video effect. */
export function scenesUsingVideo(id: string): Scene[] {
  return project()?.scenes.filter((s) => s.video?.videos.includes(id) || s.videoEffects?.includes(id)) ?? [];
}

/** Videos of a kind (null: both) in a folder (and its subfolders with `deep`), matching a
 *  search, sorted by name. */
export function findVideos(kind: VideoKind | null, folder: string | null, search: string, deep = true): Video[] {
  const words = search.toLowerCase().split(/\s+/).filter(Boolean);
  return (project()?.videos ?? [])
    .filter((v) => {
      if (kind && kindOf(v) !== kind) return false;
      if (folder !== null) {
        const f = folderOfVideo(v);
        if (deep ? !(folder === "" || f === folder || f.startsWith(`${folder}/`)) : f !== folder) return false;
      }
      const name = v.name.toLowerCase();
      return words.every((w) =>
        w.startsWith("#") ? (v.tags ?? []).some((t) => t.startsWith(w.slice(1))) : name.includes(w) || (v.tags ?? []).includes(w),
      );
    })
    .sort((a, b) => a.name.localeCompare(b.name));
}

/** Opens the file picker and copies the chosen videos into the project, as `kind`. Returns
 *  their ids. */
export async function addVideoFiles(kind: VideoKind, folder: string | null = null): Promise<string[]> {
  const title = kind === "effect" ? "Add video effects" : "Add ambience videos";
  const picked = await open({ multiple: true, title, filters: [{ name: "Video", extensions: VIDEO_EXTENSIONS }] });
  if (!picked || picked.length === 0) return [];
  return (await run(api.addVideos(kind, picked, folder))) ?? [];
}

/** Changes videos' kind and/or folder, first warning about scenes they'd leave (an ambience
 *  video turned into an effect leaves the playlists it was in), as for sounds. */
export async function moveVideos(ids: string[], kind: VideoKind, folder: string): Promise<boolean> {
  const p = project();
  if (!p) return false;
  const losing = p.scenes
    .filter((s) =>
      ids.some((id) =>
        kind === "effect" ? s.video?.videos.includes(id) && kindOf(videoById(id)!) !== "effect" : s.videoEffects?.includes(id) && kindOf(videoById(id)!) !== "ambience",
      ),
    )
    .map((s) => s.name);
  if (losing.length > 0) {
    const answer = await ask({
      title: "Change type?",
      message: `${ids.length === 1 ? "This video" : "These videos"} will be removed from: ${losing.join(", ")}.`,
      choices: [
        { label: "Cancel", value: "cancel" },
        { label: "Change type", value: "ok", primary: true },
      ],
    });
    if (answer?.choice !== "ok") return false;
  }
  await run(api.moveVideos(ids, kind, folder));
  return true;
}

/** Removes a video, asking whether to trash or delete its file. */
/** Duplicates a video like a sound (decision 0043) and shows the copy's page in the Library. */
export async function duplicateVideo(id: string) {
  const copyId = await run(api.duplicateVideo(id));
  if (!copyId) return;
  notify(`Duplicated “${videoById(id)?.name ?? "the video"}”.`);
  ui.library.selected = [copyId];
}

export async function removeVideo(id: string): Promise<boolean> {
  const video = videoById(id);
  if (!video) return false;
  const usedIn = scenesUsingVideo(id).map((s) => s.name);
  const answer = await ask({
    title: `Remove “${video.name}”?`,
    message:
      (usedIn.length ? `It's used in: ${usedIn.join(", ")}. It will be removed from them.\n\n` : "") +
      "Its file can go to the system trash (you can restore it from there) or be deleted for good.",
    choices: [
      { label: "Cancel", value: "cancel" },
      { label: "Delete permanently", value: "delete", danger: true },
      { label: "Move to trash", value: "trash", primary: true },
    ],
  });
  if (answer?.choice === "trash") await run(api.removeVideo(id, false));
  if (answer?.choice === "delete") await run(api.removeVideo(id, true));
  return answer?.choice === "trash" || answer?.choice === "delete";
}

export async function relinkVideo(id: string) {
  const name = videoById(id)?.name ?? "video";
  const path = await open({ title: `Locate the video for “${name}”`, filters: [{ name: "Video", extensions: VIDEO_EXTENSIONS }] });
  if (typeof path === "string") await run(api.relinkVideo(id, path));
}

/** "1920×1080 · VP9 · 12:40"-style description, or null before it's been measured. */
export function describeVideo(video: Video, formatDuration: (ms: number) => string): string | null {
  const i = video.info;
  if (!i) return null;
  return `${i.width}×${i.height} · ${i.codec.toUpperCase()} · ${formatDuration(i.durationMs)}`;
}
