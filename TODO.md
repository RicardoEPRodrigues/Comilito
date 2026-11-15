# TODO

## Feature

- flatpak version (not now)
- Create a first time experience.
- Add ability to have plugins.
- Add a plug in that associates a video with a scene and the top scene's video plays on a seperate window. some ideas to complement:
  - we can have a list similar to the songs where we might want to shuffle.
  - when playing this list, we might want to move between videos before they reach its end. play 5 minutes of one, then switch every 5 minutes going back to the previous point it was at. All this should be editable in settings.
  - a button to show the video playback should be added to the main UI.
  - new section should be added below the effects section (or tab) where we could play a video effect. This video would replace the currently playing video and when it is finished we go back to the playlist.
- add ability to scale the UI. Font and whole UI.

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
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- macOS and Android are listed as planned in the README. Android also needs checking that kira's
  cpal backend works there (decision 0002).

## Left to do or to check (added by Claude)

### Documented but not built yet

### Docs out of date

### Other checks

- Moving the projects folder across disks (the slow, copying case) was only tested by unit
  tests; on the virtual display both folders were on one disk, so the move was an instant rename.
  Worth trying once with a big project and a USB drive to see the progress dialog.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
