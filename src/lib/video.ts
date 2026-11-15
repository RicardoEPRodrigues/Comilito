// Videos in the Library (decision 0035): finding them, adding and removing files, and the
// URL the webview plays them from. Kept apart from the sound code, like src-tauri/src/video.rs.

import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { ask, project, run, ui } from "./store.svelte";
import type { Scene, Video } from "./types";

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

/** The Library folder a video is in: the path between `video/` and the file name. */
export function folderOfVideo(video: Video): string {
  const parts = video.file.path.split("/");
  return parts.length <= 2 ? "" : parts.slice(1, -1).join("/");
}

/** The URL the webview loads a video from: the asset protocol, limited by Rust to the open
 *  project's folder (`video::allow_media`). */
export function videoSrc(video: Video): string | null {
  const dir = ui.snapshot?.projectDir;
  if (!dir) return null;
  const sep = dir.includes("\\") ? "\\" : "/";
  return convertFileSrc(`${dir}${sep}${video.file.path.split("/").join(sep)}`);
}

/** Scenes that show a video, in their playlist or as a video effect. */
export function scenesUsingVideo(id: string): Scene[] {
  return project()?.scenes.filter((s) => s.video?.videos.includes(id) || s.videoEffects?.includes(id)) ?? [];
}

/** Videos in a folder (and its subfolders with `deep`), matching a search, sorted by name. */
export function findVideos(folder: string | null, search: string, deep = true): Video[] {
  const words = search.toLowerCase().split(/\s+/).filter(Boolean);
  return (project()?.videos ?? [])
    .filter((v) => {
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

/** Opens the file picker and copies the chosen videos into the project. Returns their ids. */
export async function addVideoFiles(folder: string | null = null): Promise<string[]> {
  const picked = await open({ multiple: true, title: "Add videos", filters: [{ name: "Video", extensions: VIDEO_EXTENSIONS }] });
  if (!picked || picked.length === 0) return [];
  return (await run(api.addVideos(picked, folder))) ?? [];
}

/** Removes a video, asking whether to trash or delete its file. */
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
