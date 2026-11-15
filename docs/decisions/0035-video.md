# 0035. Videos for scenes, in a window of their own

- Status: Accepted
- Date: 2026-11-15

## Context

A DM with a TV or projector at the table wants pictures to go with the sound: a castle gate at
dusk while the Castle scene plays, a storm when the Storm scene goes on top, a lightning strike
on cue. The owner's notes (TODO.md) ask for: a video playlist per scene (with shuffle), switching
between videos every few minutes and coming back to where each one was, video effects that
replace the playing video until they end, a button to show the video, and a video downloader
like the audio one. A video's own sound is never played.

The design is in `pencil/video-player.pen` (eight screens with notes).

## Options considered

- **A plugin vs a built-in feature.** There is no plugin system yet, and videos reach into the
  save format, the stack rules, Space and Esc, the scene editor, export, backups, relinking and
  the downloader. A plugin API covering all that would expose most of the app, for one user.
  Built in, with its own folders and data keys, it can still be moved into a plugin later.
- **Playing video in a webview vs an external player (mpv).** mpv plays every format, but its
  window lives outside the app and must be driven over a socket. A webview `<video>` is part
  of the app, but on Linux WebKitGTK plays video through GStreamer, and H.264 MP4 only plays if
  the right GStreamer packages are installed. WebM (VP8/VP9/AV1) plays out of the box.
- **Converting videos that may not play:** never (tell the user to install codecs), always, or
  offer it.
- **Videos inside the project folder or linked from elsewhere.**

## Decision

- **A built-in feature that can be turned off** (Preferences › Video › Use videos, per
  computer). When off, everything video is hidden; the project's video data is kept as is.
- **Played in a webview window of its own** (`video`), muted. Decision 0002 (all audio in Rust)
  still holds: the webview never makes a sound. The window has no title bar; its top bar, shown
  while the pointer is over it, is the drag handle, and it remembers its screen and fullscreen.
- **Videos that may not play are offered for conversion** to WebM (VP9, no sound) with the
  ffmpeg Comilito already downloads: when added, dropped, downloaded, or found in the project's
  `video/` folder. ffprobe decides what "may not play" means: anything but WebM with VP8, VP9 or
  AV1. The answer can be set to always or never in Preferences.
- **Videos are copied into the project**, under `video/<folder>/`, like audio under `audio/`.
  Export and backups get their own "include videos" choice (off by default: videos are big).
- **The same rule as music decides what plays:** the top-most enabled scene with videos plays
  its playlist. A scene without videos leaves the one below playing, unless its "hide the video
  below" switch is on. Esc (stop all) blanks the window; Space pauses it with the sound.
- **Playlist:** shuffle, "switch every N minutes" (0 = play each to its end) and "resume where it
  left off" when a video comes back. Scenes inherit these from the project defaults, like Loop
  and Fade. Where each video stopped is remembered while the app runs, not saved (TODO.md).
- **Video effects:** a list per scene, shown under the sound effects (the two share the right
  column; video effects take the height they need). Playing one replaces the scene's video
  until it ends, then the playlist resumes.
- **One Video category in the Library**, with folders, for both playlists and effects.
- **The downloader** gets an Audio / Video choice. Video downloads take the picture only, up to a
  chosen resolution (1080p by default), preferring WebM; MP4-only sources are converted after.

## Data model

In `comilito.json` (optional fields, so no `formatVersion` change; see save-format.md):

- `videos`: a list like `sounds`: `id`, `name`, `file` (path under `video/`), `tags`,
  `favourite`, `source`, `lastUsed`, and `info` (container, codec, size in pixels, length and
  whether it plays here), measured in the background with ffprobe.
- On a scene: `video` (`{ videos, shuffle?, switchMinutes?, resume? }`, or absent),
  `hidesVideo` and `videoEffects`.
- `defaults.video`: `{ shuffle, switchMinutes, resume }`; `backups.includeVideo`.

In `preferences.json`: a `video` section (on/off, screen, picture fit, crossfade, fullscreen,
download resolution, what to do with videos that may not play).

## Consequences

- A second window means a second webview, with its own capability entry (permissions).
- Local video files reach the webview through Tauri's asset protocol (the `protocol-asset`
  feature of the `tauri` crate), limited to the project folder. It supports range requests,
  which seeking in a video needs.
- Converting is slow (minutes for a long 1080p video); it runs in the background, one file at a
  time, like downloads.
- Not in the first version: trimming videos, remembering positions across restarts (TODO.md).
