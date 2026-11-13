# TODO

## Feature

- AppImage build
- flatpak version (not now)

## Left to do or to check (added by Claude)

- Downloads of only part of a video can't report a percentage (ffmpeg does the download), so they
  show "downloading the selected part…" instead. Consider estimating progress from ffmpeg's
  `time=` output.
- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- **Check on your desktop:** drop one and several files from Nautilus onto the window. It was
  tested on a virtual display with a stand-in drag source, not with a real file manager.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
