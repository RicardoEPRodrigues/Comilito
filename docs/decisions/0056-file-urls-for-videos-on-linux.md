# 0056. Play videos from `file://` URLs on Linux

- Status: Accepted (supersedes [0037](0037-streaming-videos-on-linux.md))
- Date: 2026-11-20

## Context

Decision 0037 streamed videos into a MediaSource by hand on Linux, because WebKitGTK's
`<video>` wouldn't play `asset://` URLs, stalled on `http://` ones and refused `file://` ones.
That took about 500 lines (`videoStream.ts`, `webmIndex.ts`): range requests, a buffer window,
a WebM Cues reader to jump, the header appended again after every `abort`. Jumps (scenes
resuming a video) went through the most fragile part of it, and the owner saw slow scene
switches.

Why `asset://` fails, from WebKit's source (WebKitGTK 2.50, `WebKitWebSourceGStreamer.cpp`):
WebKit loads media URLs through its own GStreamer element, `webkitwebsrc`, which accepts only
`http`, `https` and `blob`. No setting in Tauri or WebKit changes that; a custom scheme can't
reach `<video>`.

`file://` URLs don't go through `webkitwebsrc`: GStreamer opens them itself (`filesrc`), reading
straight from disk, with its own seeking. They were refused only by WebKit's security check: a
page may show local files only if its own scheme is "local". WebKitGTK lets an app register a
scheme as local (`webkit_security_manager_register_uri_scheme_as_local`).

## Options considered

- **Keep the MediaSource streamer:** works, but it's the most intricate code in the frontend
  and the part behind slow jumps.
- **Patch WebKit or ship our own `webkitwebsrc`:** out of reach for an app.
- **`file://` URLs, with `tauri` registered as a local scheme:** tested headless. A `<video>`
  played from `file://` and jumped in about 50 ms. With the scheme local, the page can *show*
  local files (`<video>`, `<img>`) but still not *read* them: `fetch` and XHR of `file://` fail
  and an image's pixels are tainted. The app never reads video pixels.

## Decision

On Linux, `loadVideo` (`src/lib/videoSource.ts`) sets the `<video>`'s `src` to the file's
`file://` URL (each path part percent-encoded); the `video_file_urls` command says when. At
startup and when the video window opens, Rust registers `tauri` as a local scheme for the
window's WebContext (`allow_local_media` in `video.rs`), through the `webkit2gtk` crate Tauri
already builds on. Debug builds also register `http`, the scheme of the Vite dev server
`tauri dev` loads pages from. Windows and macOS are unchanged (asset URLs).

## Consequences

- `videoStream.ts` and `webmIndex.ts` are gone, and so is the 32 MB give-up of decision 0041
  (a video WebKit can't decode now fails with an error at once).
- Any format the installed GStreamer plays now works on Linux, not only WebM. Conversion to WebM
  stays (decision 0035): the AppImage carries only WebM's plugins, and it's what plays everywhere.
- A page of ours could show any local file the user can read. Only our own pages load in these
  windows (no remote content), and reading the contents stays blocked.
- In `tauri dev`, any `http://` page in the webview is treated as local; release builds only
  register `tauri`.
