// Playing a project's video in a <video> element (decision 0037).
//
// On Linux, WebKitGTK refuses `<video src="asset://…">` ("source not supported") and stalls on
// `http://` media, but `fetch` of an asset URL works, range requests included, and so does
// MediaSource (MSE). So there the video is streamed by hand: read the file in chunks with
// `fetch` and append them to a SourceBuffer, keeping about a minute and a half ahead of the
// playhead and dropping what's well behind, so a long video never sits whole in memory.
// Elsewhere (Windows, macOS) a plain `src` works and is used.
//
// Seeking far (starting in the middle, resuming) uses the file's seek index when it has one
// (webmIndex.ts) to jump straight to the right place; without one, it reads forward to it.
//
// MSE takes WebM as it is (MP4 would need to be "fragmented"), so only WebM is streamed; other
// videos fall back to a plain `src`, which on Linux won't play: they're the ones offered for
// conversion anyway.

import { convertFileSrc } from "@tauri-apps/api/core";
import { ui } from "./store.svelte";
import type { Video } from "./types";
import { offsetFor, readWebmIndex, type WebmIndex } from "./webmIndex";

/** Read size per request. The asset protocol answers at most about 1 MB per range request
 *  (1,024,000 bytes), whatever is asked, so asking for more only costs memory. */
const CHUNK = 1024 * 1024;
/** Seconds to keep buffered ahead of the playhead, and behind it before dropping. */
const AHEAD = 90;
const BEHIND = 20;
/** Bytes to append before giving up on a video that never shows its metadata. The header and
 *  first cluster of a WebM are well under this; when WebKit can't read the video at all (its
 *  GStreamer plugins are missing), it would otherwise read the whole file, chunk by chunk. */
const NO_METADATA_LIMIT = 32 * 1024 * 1024;

export interface Playback {
  /** Resolves once the video can show its first frame (at the start position). */
  ready: Promise<void>;
  /** Stops loading and frees the element (it can be given another video afterwards). */
  destroy: () => void;
}

/** The absolute path of a video file, for the asset protocol. */
function filePath(video: Video): string | null {
  const dir = ui.snapshot?.projectDir;
  if (!dir) return null;
  const sep = dir.includes("\\") ? "\\" : "/";
  return `${dir}${sep}${video.file.path.split("/").join(sep)}`;
}

/** The MSE type of a WebM video, or null if it can't be streamed (not WebM, or not supported). */
function mseType(video: Video): string | null {
  if (video.file.path.split(".").pop()?.toLowerCase() !== "webm" || typeof MediaSource === "undefined") return null;
  const codec = video.info?.codec;
  const codecs = codec === "vp8" ? "vp8" : codec === "av1" ? "av01.0.08M.08" : "vp9";
  const type = `video/webm; codecs="${codecs}"`;
  return MediaSource.isTypeSupported(type) ? type : null;
}

/**
 * Loads `video` into `el`, positioned at `fromSec`. It doesn't start playing: call `el.play()`.
 * With `preview`, only enough for the first frame is read (thumbnails).
 */
export function loadVideo(el: HTMLVideoElement, video: Video, fromSec = 0, preview = false): Playback {
  const path = filePath(video);
  if (!path) return { ready: Promise.reject(new Error("no project")), destroy: () => {} };
  const url = convertFileSrc(path);
  const type = ui.videoStreaming ? mseType(video) : null;
  return type ? new Stream(el, url, type, fromSec, preview) : plain(el, url, fromSec);
}

/** A plain `src` (Windows, macOS, and videos that can't be streamed). */
function plain(el: HTMLVideoElement, url: string, fromSec: number): Playback {
  el.src = url;
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
        // A position past the end (the file changed since) starts over.
        const at = fromSec > 0 && fromSec < el.duration - 1 ? fromSec : 0;
        if (at === 0) return el.readyState >= 2 ? resolve() : el.addEventListener("loadeddata", () => resolve(), { once: true });
        el.addEventListener("seeked", () => resolve(), { once: true });
        el.currentTime = at;
      },
      { once: true },
    );
  });
}

/** Waits until a SourceBuffer has finished its current append or remove. */
function idle(sb: SourceBuffer): Promise<void> {
  return sb.updating ? new Promise((r) => sb.addEventListener("updateend", () => r(), { once: true })) : Promise.resolve();
}

/** A video streamed into a MediaSource (Linux). */
class Stream implements Playback {
  ready: Promise<void>;
  private ms = new MediaSource();
  private objectUrl: string;
  private sb: SourceBuffer | null = null;
  private size = 0;
  /** Where the next chunk starts in the file. */
  private offset = 0;
  private feeding = false;
  private dead = false;
  /** Set by a seek outside the buffered data: drop what's buffered and continue reading from
   *  this byte (a cluster from the seek index, or 0 without one). Done by the reading loop
   *  between chunks, so it never races a chunk being appended. */
  private reposition: number | null = null;
  /** The file's seek index, if it has one: lets seeks jump straight to the right place. */
  private index: WebmIndex | null = null;
  /** Bytes appended so far, to notice a video that never gets its metadata. */
  private appended = 0;
  private fail: (e: Error) => void = () => {};

  constructor(
    private el: HTMLVideoElement,
    private url: string,
    private type: string,
    private fromSec: number,
    private preview: boolean,
  ) {
    this.objectUrl = URL.createObjectURL(this.ms);
    const failed = new Promise<never>((_, reject) => (this.fail = reject));
    this.ready = Promise.race([whenLoaded(el, fromSec), failed]);
    this.ms.addEventListener("sourceopen", () => this.open(), { once: true });
    el.addEventListener("timeupdate", this.pump);
    el.addEventListener("seeking", this.onSeeking);
    el.src = this.objectUrl;
  }

  private async open() {
    if (this.dead) return;
    const sb = (this.sb = this.ms.addSourceBuffer(this.type));
    // The file's size, from the answer to a one-byte range request.
    const head = await fetch(this.url, { headers: { Range: "bytes=0-0" } });
    const size = Number(head.headers.get("content-range")?.split("/")[1] ?? 0);
    this.index = await readWebmIndex(this.readRange, size).catch(() => null);
    if (this.dead) return;
    // Starting in the middle: the header first (MediaSource needs it before any video), then
    // straight to the cluster holding the start, instead of reading everything before it.
    if (this.index && this.fromSec > 0) {
      sb.appendBuffer(await this.readRange(0, this.index.initEnd - 1));
      await idle(sb);
      this.offset = offsetFor(this.index, this.fromSec);
    }
    this.size = size;
    this.pump();
  }

  /** Bytes [start, end] of the file, in as many requests as the asset protocol needs (it
   *  answers at most about 1 MB per request). */
  private readRange = async (start: number, end: number): Promise<Uint8Array<ArrayBuffer>> => {
    const out = new Uint8Array(end - start + 1);
    let at = start;
    while (at <= end) {
      const response = await fetch(this.url, { headers: { Range: `bytes=${at}-${end}` } });
      const part = new Uint8Array(await response.arrayBuffer());
      if (part.length === 0) break;
      out.set(part, at - start);
      at += part.length;
    }
    return out;
  };

  /** The seconds buffered ahead of the playhead (0 if the playhead isn't in buffered data). */
  private ahead(): number {
    const b = this.el.buffered;
    const t = this.el.currentTime;
    for (let i = 0; i < b.length; i++) if (b.start(i) <= t + 0.1 && t <= b.end(i)) return b.end(i) - t;
    return 0;
  }

  private needsMore(): boolean {
    // Before the metadata there's no playhead yet: read until there is.
    if (this.el.readyState < 1) return true;
    return this.ahead() < (this.preview ? 1 : AHEAD);
  }

  /** Reads and appends chunks until enough is buffered ahead. Called again on `timeupdate`. */
  private pump = async () => {
    const sb = this.sb;
    if (this.feeding || this.dead || !sb || !this.size) return;
    this.feeding = true;
    try {
      while (!this.dead && (this.reposition !== null || (this.offset < this.size && this.needsMore()))) {
        if (this.reposition !== null) {
          const to = this.reposition;
          this.reposition = null;
          await idle(sb);
          // The parser may be halfway through a cluster from the last chunk (chunks are cut
          // anywhere): `abort` resets it, or the bytes from the new place would be read as the
          // rest of that cluster and fail to decode. Once the whole file was read (`ended`)
          // there's nothing half-read, and `abort` would throw: `remove` reopens the source.
          if (this.ms.readyState === "open") sb.abort();
          if (this.el.buffered.length) sb.remove(0, Infinity);
          await idle(sb);
          this.offset = to;
          continue;
        }
        await this.dropBehind(sb);
        const wanted = Math.min(this.size, this.offset + CHUNK) - 1;
        const response = await fetch(this.url, { headers: { Range: `bytes=${this.offset}-${wanted}` } });
        const chunk = await response.arrayBuffer();
        if (this.dead) return;
        // The server may send less than asked (the asset protocol caps each answer): carry on
        // from what actually came, or bytes would be skipped and the video cut short.
        if (chunk.byteLength === 0) break;
        const end = this.offset + chunk.byteLength - 1;
        await idle(sb);
        try {
          sb.appendBuffer(chunk);
        } catch (e) {
          // Full: drop more of what's behind and try this chunk again on the next timeupdate.
          if ((e as DOMException).name === "QuotaExceededError") {
            await this.dropBehind(sb, 2);
            return;
          }
          throw e;
        }
        await idle(sb);
        this.offset = end + 1;
        this.appended += chunk.byteLength;
        if (this.el.readyState < 1 && this.appended > NO_METADATA_LIMIT) {
          this.dead = true;
          this.fail(new Error("the video can't be read here (are WebKit's GStreamer plugins installed?)"));
          return;
        }
      }
      if (!this.dead && this.offset >= this.size && this.ms.readyState === "open") {
        await idle(sb);
        this.ms.endOfStream();
      }
    } catch (e) {
      if (!this.dead) console.warn("video stream:", e);
    } finally {
      this.feeding = false;
    }
  };

  /** Removes buffered data more than `keep` seconds behind the playhead. */
  private async dropBehind(sb: SourceBuffer, keep = BEHIND) {
    const until = this.el.currentTime - keep;
    if (this.el.buffered.length && this.el.buffered.start(0) < until) {
      await idle(sb);
      sb.remove(0, until);
      await idle(sb);
    }
  }

  /** A seek outside the buffered data. With a seek index, jump straight to the cluster holding
   *  that point. Without one, a seek back (looping, resuming earlier) reads again from the
   *  start, and a seek ahead keeps reading forward until it gets there. */
  private onSeeking = () => {
    const b = this.el.buffered;
    const t = this.el.currentTime;
    let buffered = false;
    for (let i = 0; i < b.length; i++) if (b.start(i) <= t + 0.1 && t <= b.end(i)) buffered = true;
    if (!buffered) {
      if (this.index) this.reposition = offsetFor(this.index, t);
      else if (b.length && t < b.start(0)) this.reposition = 0;
    }
    this.pump();
  };

  destroy() {
    this.dead = true;
    this.el.removeEventListener("timeupdate", this.pump);
    this.el.removeEventListener("seeking", this.onSeeking);
    this.el.removeAttribute("src");
    this.el.load();
    URL.revokeObjectURL(this.objectUrl);
  }
}

/**
 * A Svelte action for simple cases (`<video use:videoSource={{ video, at }}>`): the Library's
 * preview and thumbnails. `at` is where to show (seconds); `preview` reads only enough for the
 * first frame.
 */
export function videoSource(el: HTMLVideoElement, params: { video: Video; at?: number; preview?: boolean }) {
  let current = loadVideo(el, params.video, params.at ?? 0, params.preview);
  current.ready.catch(() => {});
  return {
    update(next: { video: Video; at?: number; preview?: boolean }) {
      if (next.video.file.path === params.video.file.path) return;
      params = next;
      current.destroy();
      current = loadVideo(el, next.video, next.at ?? 0, next.preview);
      current.ready.catch(() => {});
    },
    destroy() {
      current.destroy();
    },
  };
}
