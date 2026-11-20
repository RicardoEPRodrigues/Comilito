# 0041. Bundle a chosen set of GStreamer plugins in the AppImage

- Status: Accepted
- Date: 2026-11-15

## Context

WebKitGTK plays `<video>` through GStreamer. The AppImage bundler copies GStreamer's libraries
into the AppImage (WebKit links to them) but not its plugins. Ubuntu's GStreamer is
relocatable: it looks for plugins next to `libgstreamer-1.0.so`, which inside the AppImage is
an empty place. So the AppImage found no plugins at all ("GStreamer element appsink not
found"), couldn't play any video, and opening a project with videos froze it. Under
`tauri dev`, and in the `.deb`, the system's GStreamer and plugins are used and all is well.

It also wrote that empty plugin list to the user's shared registry
(`~/.cache/gstreamer-1.0/registry.x86_64.bin`), so other GStreamer programs had to rescan.

## Options considered

- **Point the AppImage at the system's plugins** (`GST_PLUGIN_SYSTEM_PATH`): tiny, but the
  system's plugins are built against the system's GStreamer, which on another distro may be
  newer than the bundled one; plugins would fail to load. Defeats the point of an AppImage.
- **Tauri's `bundleMediaFramework`, as is:** official, but copies every plugin installed on
  the build machine, here including FFmpeg (libav), plugins-ugly and VA-API: a much larger
  AppImage carrying patented codecs (H.264, HEVC) we don't need, since videos that aren't WebM
  are converted (decision 0035).
- **`bundleMediaFramework` with a chosen plugin list:** the bundler reads the plugins from
  `GSTREAMER_PLUGINS_DIR` when set, so a script can hand it only what WebM playback needs.

## Decision

`bundleMediaFramework` is on (in `tauri.linux.conf.json`), and `npm run tauri` goes through
`scripts/tauri.mjs`. On Linux, for a `build` that makes an AppImage, it copies about 37 plugins
into a temporary folder and points the bundler at it (`GSTREAMER_PLUGINS_DIR`): GStreamer's core
elements, plugins-base, OpenGL, ALSA, and from plugins-good/bad only Matroska/WebM, VP8/VP9,
AV1 (aom), parsers, audio outputs and a few video filters. It stops with a clear message if
`patchelf` (which the bundler needs) or a required plugin is missing, so every AppImage built
the usual way has the right plugins. Other commands, `--no-bundle` builds and other OSes go
straight to the Tauri CLI. The wrapper is Node, not a shell script, so it works on Windows.

Running the Tauri CLI directly (`npx tauri build`) skips the wrapper and bundles every
installed plugin.

At startup inside an AppImage (`APPIMAGE` set), Comilito sets `GST_REGISTRY_1_0` to a file in
`~/.cache/me.colorcrow.comilito/`, so its plugin list stays out of the shared registry.

The video streamer also gives up on a video once 32 MB have been appended without WebKit
reading its metadata, instead of reading the whole file.

## Consequences

- The AppImage grows by a few MB (the plugins and the libraries they need, e.g. libvpx).
- Only WebM plays in the AppImage; that's already all that plays everywhere (decision 0035).
- `patchelf` is a build requirement on Linux (in the README's prerequisites).
- The plugin list (in `scripts/tauri.mjs`) may need an addition if WebKit starts asking for another element; its
  stderr (in the logs folder) names missing elements.
