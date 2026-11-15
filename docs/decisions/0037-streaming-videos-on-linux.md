# 0037. Streaming videos into a MediaSource on Linux

- Status: Accepted (amends how [0035](0035-video.md) loads videos)
- Date: 2026-11-15

## Context

Decision 0035 planned to play the project's videos with `<video src="asset://…">`. On Linux
(WebKitGTK 2.52, GStreamer 1.24, tested with WebDriver, decision 0036) that never played:

- `asset://` URLs: the `<video>` fails at once with error 4, "source not supported", although
  `fetch` of the same URL works, range requests included (206, `video/webm`).
- `http://127.0.0.1` URLs (a small server in the app, and Python's `http.server` to rule the
  server out): the whole file downloads, metadata loads, then playback stalls 0.02 s in, with
  the network state stuck at "loading", whatever the `preload`. Hardware decoding was ruled out.
- `file://` URLs: refused.
- A `blob:` URL of the whole file plays, so decoding works, but it needs the whole file in
  memory, and videos can be gigabytes.
- MediaSource (MSE) fed by `fetch` over `asset://` plays, seeks and loops.

Windows (WebView2) and macOS (WKWebView) play from asset URLs directly.

## Options considered

- **A local HTTP server:** built and tested; WebKitGTK stalls on HTTP media here, so it doesn't
  help, and it would add a listening socket.
- **Blob URLs:** simple, but whole files in memory.
- **MSE streaming:** the frontend reads the file in pieces with `fetch` (asset protocol) and
  appends them to a SourceBuffer. More code, but bounded memory and no new attack surface.

## Decision

- On Linux (`video_streaming` command), videos are streamed into a MediaSource
  (`src/lib/videoStream.ts`): about 1 MB per range request, about 90 s kept buffered ahead
  of the playhead, data more than 20 s behind dropped. A seek ahead keeps reading forward; a seek
  before the buffered data (looping, resuming earlier) reads again from the start, for files
  without a seek index (see below).
- The asset protocol answers at most 1,024,000 bytes per range request whatever is asked, so the
  next request starts where the last answer ended (asking for 1 MB and assuming it came skipped
  bytes and cut the video in half).
- Only WebM is streamed: MSE needs MP4 to be fragmented. Other videos fall back to a plain `src`,
  which on Linux won't play; they're the ones offered for conversion (0035) anyway.
- Windows and macOS keep a plain `src` with the asset URL.
- Seeking uses the WebM's seek index (its Cues) when it has one (`src/lib/webmIndex.ts`):
  starting in the middle appends the header, then reads from the cluster holding that point; a
  seek outside the buffered data jumps the same way (resetting the SourceBuffer's parser with
  `abort()` first, since chunks are cut mid-cluster). Comilito's conversions write the Cues at
  the front (`-cues_to_front 1`); other files usually have them at the end, found through the
  SeekHead. Tested: seeking to 2:30 of a 3-minute 720p video read 4 MB in about 40 ms, either way.

## Consequences

- A WebM without Cues (rare) still reads forward from the start to reach a point in the middle.
- Thumbnails stream only enough for one frame.
- No HTTP server, so nothing listens on the network.
