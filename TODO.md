# TODO

## From the developer

- flatpak version

## Left to do or to check (added by Claude)

- **Check in the real app:** dragging a sound from the Library drawer onto a scene in the
  Tapestry, onto a scene card in the Scenes tab, or into the scene editor. The hover state now
  works ("Release to add … to …"); the drop itself couldn't be confirmed before testing stopped.
- **Check in the real app:** the waveform cursor during a *crop* preview. kira reports positions
  inside a slice, so the code adds the slice's start; confirm the cursor lines up with the audio.
- **Check after reinstalling:** the desktop menu icon is no longer blurry. 48, 256 and 512 px icons
  were added to the bundle, so menus no longer scale up the 32 px one.
- Downloads of only part of a video can't report a percentage (ffmpeg does the download), so they
  show "downloading the selected part…" instead. Consider estimating progress from ffmpeg's
  `time=` output.
- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
