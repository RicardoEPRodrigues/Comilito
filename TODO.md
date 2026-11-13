# TODO

## Feature

- AppImage build
- flatpak version (not now)
- Redesign the home screen with pencil. there should be an option somewhere to open the previous project at start. also, I want to be able to duplicate projects. the import and restore can be hidden under a drop down menu.
- Settings: option to migrate the projects folder.

## Left to do or to check (added by Claude)

- Downloads of only part of a video can't report a percentage (ffmpeg does the download), so they
  show "downloading the selected part…" instead. Consider estimating progress from ffmpeg's
  `time=` output.
- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
