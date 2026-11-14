# TODO

## Feature

- flatpak version (not now)
- Add ability to have plugins.
- Add a plug in that associates a video with a scene and the top scene's video plays on a seperate window.
- Create a first time experience.
- Resume unfinished downloads after new launch.
- add animations, so opening and closing menus is smoother. e.g., opening the library drawer.
- add a keybind (default backspace) to delete the top most scene from the stack.
- make the middle mouse click disable the selected scene.
- allow library drawer to be resizable.

## Bugs

- If I play a sound on its edit page and then leave, the sound keeps playing. we should create an area where we can stop any temporary sounds.
- in the youtube video downloader, the input for the beginning and end of the video are not that pretty and are overflowing. I prefer the 00:00:00 format for example.
- Library: closing a folder does not hide subfolders.
- Not able to drag a scene into a group.
- any new ambient sound added to a scene should be labeled as foreground. Remove the No Role option. allow editing the role from the tapestry.
- the decorated logo is too loud, lets switch to use the mark icon for everything (lets keep the decorated icon saved for later use).


## Platforms

- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows: the desktop accent colour (registry) and the downloaded tools are untested.
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
