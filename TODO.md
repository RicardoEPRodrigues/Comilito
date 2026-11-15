# TODO

## Feature

- flatpak version (not now)
- Create a first time experience.
- add ability to scale the UI. Font and whole UI.
- Add ability to have plugins.
- Add a plug in that associates a video with a scene and the top scene's video plays on a seperate window. some ideas to complement:
  - Status: built as a feature that can be turned off (not a plugin), on the `video` branch,
    waiting for manual testing (see "Other checks"). Design: pencil/video-player.pen;
    docs/video.md.
  - the video audio is never played.
  - we can have a list similar to the songs where we might want to shuffle.
  - when playing this list, we might want to move between videos before they reach its end. play 5 minutes of one, then switch every 5 minutes going back to the previous point it was at. All this should be editable in settings.
  - a button to show the video playback should be added to the main UI.
  - new section should be added below the effects section (or tab) where we could play a video effect. This video would replace the currently playing video and when it is finished we go back to the playlist.
  - Video downloader akin to the audio downloader.
  - Later, not in the first version (from the design review):
    - Trimming videos (like cropping sounds).
    - Remembering where each video left off across restarts (the first version remembers it
      only while the app is open).
    - Video plugins, once plugins exist: the video feature has its own folders and data keys so
      it could be moved into one.
    - Fingerprinting videos (sha256) like audio, so "Find in a folder" can tell same-named
      videos apart; videos are relinked by name and size only for now.
    - Conversions don't resume after a restart (downloads do); the video stays "may not play"
      with its Convert button.
    - Moving the video window to another screen from Preferences only applies the next time
      it opens (its own screen picker moves it straight away).
    - The Library's "Missing files" list shows sounds only; missing videos are marked in the
      Video shelf.

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

- The whole video feature (decision 0035, `video` branch) was checked by type-checks, unit
  tests and real ffmpeg/yt-dlp runs only, never in the running app (no display here). Worth
  trying: adding an MP4 and converting it; a scene with two videos and "switch every 1 min";
  coming back to a scene (resume); a video effect; Space and Esc; the video window on a second
  screen and fullscreen; downloading a part of a YouTube video as video; exporting with videos.
- Moving the projects folder across disks (the slow, copying case) was only tested by unit
  tests; on the virtual display both folders were on one disk, so the move was an instant rename.
  Worth trying once with a big project and a USB drive to see the progress dialog.
- Flatpak needs a permanent app ID (reverse domain, e.g. `me.colorcrow.Comilito`), plus
  `flatpak-builder` and the GNOME SDK with the Rust extension installed to build and test it.
