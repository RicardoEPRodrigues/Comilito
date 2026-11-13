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
- Moving the projects folder to another disk copies everything while the app waits, with no
  progress shown (decision 0023). Fine for small projects; large ones may look frozen for a while.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
