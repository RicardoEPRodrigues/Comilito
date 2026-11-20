// Playing a project's video in a <video> element (decision 0056).
//
// Windows and macOS play the file through Tauri's asset protocol (`asset://…`). WebKitGTK
// (Linux) can't: its <video> loads only http(s), blob and file URLs, so there the element gets
// the file's `file://` URL, which GStreamer reads straight from disk. Rust lets our pages do that
// (`allow_local_media` in video.rs); it's what `ui.videoFileUrls` says.

import { convertFileSrc } from "@tauri-apps/api/core";
import { ui } from "./store.svelte";
import type { Video } from "./types";

export interface Playback {
  /** Resolves once the video can show its first frame (at the start position). */
  ready: Promise<void>;
  /** Stops loading and frees the element (it can be given another video afterwards). */
  destroy: () => void;
}

/** The absolute path of a video file. */
function filePath(video: Video): string | null {
  const dir = ui.snapshot?.projectDir;
  if (!dir) return null;
  const sep = dir.includes("\\") ? "\\" : "/";
  return `${dir}${sep}${video.file.path.split("/").join(sep)}`;
}

/** A `file://` URL for an absolute Unix path. Each part is percent-encoded, so spaces, `#`,
 *  `?`, `%` and non-ASCII names survive. */
function fileUrl(path: string): string {
  return "file://" + path.split("/").map(encodeURIComponent).join("/");
}

/** Loads `video` into `el`, positioned at `fromSec`. It doesn't start playing: call `el.play()`. */
export function loadVideo(el: HTMLVideoElement, video: Video, fromSec = 0): Playback {
  const path = filePath(video);
  if (!path) return { ready: Promise.reject(new Error("no project")), destroy: () => {} };
  el.src = ui.videoFileUrls ? fileUrl(path) : convertFileSrc(path);
  return {
    ready: whenLoaded(el, fromSec),
    destroy: () => {
      el.removeAttribute("src");
      el.load();
    },
  };
}

/** Resolves when the element has its first frame, after moving to `fromSec`. */
function whenLoaded(el: HTMLVideoElement, fromSec: number): Promise<void> {
  return new Promise((resolve, reject) => {
    el.addEventListener("error", () => reject(el.error), { once: true });
    el.addEventListener(
      "loadedmetadata",
      () => {
        if (fromSec <= 0) {
          return el.readyState >= 2 ? resolve() : el.addEventListener("loadeddata", () => resolve(), { once: true });
        }
        // A position in the last second (or past the end, if the file changed) starts over.
        const at = fromSec < el.duration - 1 ? fromSec : 0;
        el.addEventListener("seeked", () => resolve(), { once: true });
        el.currentTime = at;
      },
      { once: true },
    );
  });
}

/**
 * A Svelte action for simple cases (`<video use:videoSource={{ video, at }}>`): the Library's
 * preview and thumbnails. `at` is where to show (seconds).
 */
export function videoSource(el: HTMLVideoElement, params: { video: Video; at?: number }) {
  let current = loadVideo(el, params.video, params.at ?? 0);
  current.ready.catch(() => {});
  return {
    update(next: { video: Video; at?: number }) {
      if (next.video.file.path === params.video.file.path) return;
      params = next;
      current.destroy();
      current = loadVideo(el, next.video, next.at ?? 0);
      current.ready.catch(() => {});
    },
    destroy() {
      current.destroy();
    },
  };
}
