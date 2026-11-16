# TODO

## Feature

- flatpak version (not now)
- Add ability to have plugins (not now)
- Create a first time experience.
- add ability to scale the UI. Font and whole UI.
- Mini player: a very small version where the window remais always on top and just shows the stack and the scenes/presets.
- if not enought width is available the bottom bar squishes its elements. we should adapt the UI to play nicely on mobile.

## Bugs


## Platforms

- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Windows and macOS: choosing the audio output (decision 0032) lists devices through cpal's own
  hosts there; untested.
- Windows and macOS: the OS media controls (decision 0030) compile but are untested. On
  Windows they're tied to the main window's handle, created from a background thread.
- Windows and macOS: the progress of downloading part of a video comes from ffmpeg's status
  line, which it redraws with `\r`. Output is split at `\r` and `\n` (so Windows' `\r\n` works
  too), and ffmpeg prints the same `time=` everywhere, so it should work; untested.
- Windows and macOS: the video window (decision 0035) is untested: WebView2 and WKWebView play
  MP4 as well as WebM, the borderless window's drag handle (`data-tauri-drag-region`),
  fullscreen on a chosen screen, and the asset protocol URLs (`convertFileSrc`).
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- macOS and Android are listed as planned in the README. Android also needs checking that kira's
  cpal backend works there (decision 0002).

## Left to do or to check (added by Claude)

### Documented but not built yet

### Docs out of date

### Other checks

- Not tried yet in the running app (only type-checked and unit-tested): opening a project from
  outside the projects folder and its home-screen entry (decision 0042), duplicating a sound and
  a large video (0043), and restarting the interface with F5, including with a dialog open and
  with the video window open (0044).
- Backups are filed by project name (decision 0026), so two projects with the same name (more
  likely now that projects can live outside the projects folder, decision 0042) share a backup
  folder, and pruning to one project's "keep" count can delete the other's backups. Keying the
  folder by something unique to the project (an id in `comilito.json`) would fix it; that's a
  save-format change.
- Video (decision 0035): not tried yet in the running app: the video window on a second screen
  and in fullscreen, and exporting a project with its videos.
- AppImage videos (decision 0041): the built AppImage's GStreamer and plugins were checked with
  GStreamer's own tools (WebM decodes), not yet in the running AppImage. Open a project with
  videos in it and watch the video window; WebKit's stderr names any element still missing.
- Moving the projects folder across disks (the slow, copying case) was only tested by unit
  tests; on the virtual display both folders were on one disk, so the move was an instant rename.
  Worth trying once with a big project and a USB drive to see the progress dialog.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
