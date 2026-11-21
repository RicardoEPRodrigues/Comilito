# TODO

## Feature

### v1.0.0

### v1.1.0

- In-app auto updates.
- Create a first time experience.
- Scroll on the volume bars should increase/decrease volume.
- Allow scenes to completely mute underneath audio.

### After v1.1.0

- Add ability to have plugins (not now)
- Add sound post-effects to make sounds appear as if they are in a certain environment. Echo for caves, reverb for churches, etc.
- Adapt the UI to play nicely on mobile. A first design is in pencil/comilito.pen, section
  "Later · Mobile" (Scenes / Tapestry / Effects tabs, the mixer bar on two rows). On the desktop
  a narrow window uses the mini player (decision 0049).
- Middle clicking on a scene in the tapestry and draggin up or down, should disable or enable the scenes based on the state of the first clicked on scene. e.g., click on the a disabled scene and drag up, that scene and above scenes selected by the mouse, should now be enabled.
- Allow video effects to play audio. If they do, then there should be options to duck the music or to mute all audio while that is playing.

- Flathub refused the submission for 2 reasons: too new (they expect a repo to be at least 1 or 2 months) and the biggest issue is due to it being created with AI. Unfortunately, the second point, although not made clear in the documentation is what made the rejection final. https://github.com/flathub/flathub/pull/11289

## Bugs



## Platforms

- Windows and macOS: choosing the audio output (decision 0032) lists devices through cpal's own
  hosts there; untested.
- Windows and macOS: the progress of downloading part of a video comes from ffmpeg's status
  line, which it redraws with `\r`. Output is split at `\r` and `\n` (so Windows' `\r\n` works
  too), and ffmpeg prints the same `time=` everywhere, so it should work; untested.
- Windows and macOS: dropping audio files onto the window (decision 0022) uses a different path
  from Linux (file contents sent from the webview) and is untested.
- macOS and Android are listed as planned in the README. Android also needs checking that kira's
  cpal backend works there (decision 0002).

- macOS: choose where ffmpeg comes from (decision 0019); the app currently expects one on the PATH.
- macOS: the OS media controls (decision 0030) compile but are untested.
- macOS: the video window (decision 0035) is untested: WKWebView play
  MP4 as well as WebM, the borderless window's drag handle (`data-tauri-drag-region`),
  fullscreen on a chosen screen, and the asset protocol URLs (`convertFileSrc`).

- GPU video encoding (decision 0051) was tested only with NVENC on Linux (RTX 5070 Ti). VA-API
  (Intel and AMD on Linux), Quick Sync and AMF (Windows) are untested: their quality settings are
  a bitrate for the picture size, and a failure falls back to the CPU.

## Left to do or to check (added by Claude)

### Documented but not built yet

- Flatpak (decisions 0059, 0061, docs/flatpak.md): the Flathub manifest's offline build hasn't
  been tried yet. Once a release tag is pushed, run `npm run flatpak -- --flathub`. Only matters
  if Flathub is tried again (decision 0063).
- Flatpak: before submitting, get `colorcrow.me` ready to serve Flathub's verification token over
  HTTPS. Only matters if Flathub is tried again.
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
