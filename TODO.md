# TODO

## Feature

- flatpak version (not now)
- Add ability to have plugins.
- Add a plug in that associates a video with a scene and the top scene's video plays on a seperate window.
- Create a first time experience.
- Resume unfinished downloads after new launch.
- add buttons to open all folders and close all folders.

## Bugs


## Platforms

- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
- Windows and macOS: the OS media controls (decision 0030) compile but are untested. On
  Windows they're tied to the main window's handle, created from a background thread.
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- macOS and Android are listed as planned in the README. Android also needs checking that kira's
  cpal backend works there (decision 0002).

## Left to do or to check (added by Claude)

### Documented but not built yet

- Choosing the audio output device. It's in docs/preferences.md (Playback › Audio output device)
  and decision 0012, and the README lists it under "Not yet". The engine always opens the system
  default (`src-tauri/src/audio.rs`). Decision 0012 says to first check that kira's cpal backend
  can pick a device; it should also cope with the device disappearing (USB headset unplugged) by
  falling back to the default.
- Warning when quitting during hashing or a backup. docs/preferences.md lists a "Warn when
  quitting during hashing or backup" setting (default on) and the README lists it under "Not
  yet"; neither the setting nor the warning exists.

### Docs out of date

### Other checks

- Downloads of only part of a video can't report a percentage (ffmpeg does the download), so they
  show "downloading the selected part…" instead. Consider estimating progress from ffmpeg's
  `time=` output.
- Moving the projects folder across disks (the slow, copying case) was only tested by unit
  tests; on the virtual display both folders were on one disk, so the move was an instant rename.
  Worth trying once with a big project and a USB drive to see the progress dialog.
- Library: a folder can only be moved within its category. Moving one to another category would
  also mean moving its sounds between categories (and out of scene slots that don't take them).
- Library: dragging a sound onto a folder couldn't be checked on the virtual display (synthetic
  drags of sound rows didn't register there); folder drags did. Worth one manual try.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
- Opening a project made before roles were required (decision 0029): layers without a role
  become Foreground, and a scene with no ambience no longer ducks the ones below. Check that an
  older real project still sounds right.
