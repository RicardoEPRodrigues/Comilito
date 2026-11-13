# TODO

## Feature

- flatpak version
- when I drag a music file into the app, the app should import a copy after a confirmation prompt. (one or multiple files)

## Bugs

- the sound wave graphics doesn't always show on the app. need to play for it to appear.
- the sound wave graphic changes size when playing because the time appears. there should be no jump.

## Left to do or to check (added by Claude)

- **Check after rebuilding:** the icon in GNOME Shell (dock, Activities) is sharp. Tauri uses the
  first PNG in `bundle.icon` as the window icon; it was the 32 px one and is now 256 px.

- Downloads of only part of a video can't report a percentage (ffmpeg does the download), so they
  show "downloading the selected part…" instead. Consider estimating progress from ffmpeg's
  `time=` output.
- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
