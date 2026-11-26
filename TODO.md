# TODO

## Feature

- expand all, collapse all in the library.
- more shortcuts: double click on sound to add to the active scene; ctrl-click to enable disable
- have a library/library drawer like experience for scenes and presets.
- Hotkeys for SFX
- Touch inputs on desktop

### v1.1.0

- In-app auto updates.
- Create a first time experience.
- Scroll on the volume bars should increase/decrease volume.
- Allow scenes to completely mute underneath audio.
- Middle clicking on a scene in the tapestry and draggin up or down, should disable or enable the scenes based on the state of the first clicked on scene. e.g., click on the a disabled scene and drag up, that scene and above scenes selected by the mouse, should now be enabled.

### v1.2.0

- Allow video effects to play audio. If they do, then there should be options to duck the music or to mute all audio while that is playing.

### v1.3.0

- Add sound post-effects to make sounds appear as if they are in a certain environment. Echo for caves, reverb for churches, etc.

### v1.5.0

- Add ability to have plugins. Ideas: twitch integration; discord integration; video/audio streaming; spotify integration (play songs or playlists instead of music).

## Bugs

- On Wayland, clicking the miniplayer button does not seem to inform the window manager of the change and when moving it assumes the previous size. Only by ajusting the window size on the corners does the new size is considered.
- On Wayland always on top does not work.


## Left to do or to check (added by Claude)

### Documented but not built yet

- Releases (decision 0063): `npm run publish-release` hasn't been run for real yet, nor on
  Windows. v1.0.0 has no GitHub release: publish it on Linux and on Windows, so the website's
  download buttons work.

### Docs out of date

### Other checks

- Comilito crashes (segfault) when a file chooser returns no local path: rfd 0.17.2's GTK
  backend passes GTK's result to `CStr::from_ptr` without checking for null
  (`backend/gtk3/file_dialog/dialog_ffi.rs`). Seen in the Flatpak when the portal failed to
  register a folder; a network location (gvfs, no local path) may do the same anywhere on Linux.
  Report it upstream (rfd), or check whether a newer rfd fixes it.
- ffmpeg is pinned to BtbN's 8.1 release branch (`FFMPEG_BRANCH` in tools.rs, decision 0051)
  because the 9.0 builds' NVENC needs NVIDIA driver 610+. Raise it once that driver is common.

## Platform Specific Issues/Checks

- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- macOS: the OS media controls (decision 0030) compile but are untested.
- macOS: the video window (decision 0035) is untested: WKWebView play
  MP4 as well as WebM, the borderless window's drag handle (`data-tauri-drag-region`),
  fullscreen on a chosen screen, and the asset protocol URLs (`convertFileSrc`).
- macOS: the progress of downloading part of a video comes from ffmpeg's status
  line, which it redraws with `\r`. Output is split at `\r` and `\n` (so Windows' `\r\n` works
  too), and ffmpeg prints the same `time=` everywhere, so it should work; untested.
- macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- macOS: choosing the audio output (decision 0032) lists devices through cpal's own
  hosts there; untested.

- Adapt the UI to play nicely on mobile. A first design is in pencil/comilito.pen, section
  "Later · Mobile" (Scenes / Tapestry / Effects tabs, the mixer bar on two rows). On the desktop
  a narrow window uses the mini player (decision 0049).

- GPU video encoding (decision 0051) was tested only with NVENC on Linux (RTX 5070 Ti). VA-API
  (Intel and AMD on Linux), Quick Sync and AMF (Windows) are untested: their quality settings are
  a bitrate for the picture size, and a failure falls back to the CPU.