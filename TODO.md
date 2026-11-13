# TODO

## Feature

- AppImage build
- flatpak version (not now)
- Add ability to have plugins.
- Add a plug in that associates a video with a scene and the top scene's video plays on a seperate window.

## Bugs

## Left to do or to check (added by Claude)

- Downloads of only part of a video can't report a percentage (ffmpeg does the download), so they
  show "downloading the selected part…" instead. Consider estimating progress from ffmpeg's
  `time=` output.
- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- Moving the projects folder across disks (the slow, copying case) was only tested by unit
  tests; on the virtual display both folders were on one disk, so the move was an instant rename.
  Worth trying once with a big project and a USB drive to see the progress dialog.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
