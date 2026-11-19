# TODO

## Feature

- flatpak version (not now)
- Add ability to have plugins (not now)
- Create a first time experience.
- Adapt the UI to play nicely on mobile. A first design is in pencil/comilito.pen, section
  "Later · Mobile" (Scenes / Tapestry / Effects tabs, the mixer bar on two rows). On the desktop
  a narrow window uses the mini player (decision 0049).
- Middle clicking on a scene in the tapestry and draggin up or down, should disable or enable the scenes based on the state of the first clicked on scene. e.g., click on the a disabled scene and drag up, that scene and above scenes selected by the mouse, should now be enabled.
- Support GPU encoding.
- if two GPUs are available, add a video option for which to use.
- Improve theme:
  - Codex: header bar should follow accent color.
  - there should be a clear light theme. similar to the ironwood but light.

## Bugs

- In the library, the sounds duplicate and delete buttons are different from the videos. we should be consistent in the design. lets use the design as shown in the video editor. Also, the trash can icon should be marked as dangerous and thus have a different color.


## Platforms

- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- Windows and macOS: choosing the audio output (decision 0032) lists devices through cpal's own
  hosts there; untested.
- macOS: the OS media controls (decision 0030) compile but are untested.
- Windows and macOS: the progress of downloading part of a video comes from ffmpeg's status
  line, which it redraws with `\r`. Output is split at `\r` and `\n` (so Windows' `\r\n` works
  too), and ffmpeg prints the same `time=` everywhere, so it should work; untested.
- macOS: the video window (decision 0035) is untested: WKWebView play
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

- Backups are filed by project name (decision 0026), so two projects with the same name (more
  likely now that projects can live outside the projects folder, decision 0042) share a backup
  folder, and pruning to one project's "keep" count can delete the other's backups. Keying the
  folder by something unique to the project (an id in `comilito.json`) would fix it; that's a
  save-format change.
- Video (decision 0035): exporting a project with its videos.
- Moving the projects folder across disks (the slow, copying case) was only tested by unit
  tests; on the virtual display both folders were on one disk, so the move was an instant rename.
  Worth trying once with a big project and a USB drive to see the progress dialog.
