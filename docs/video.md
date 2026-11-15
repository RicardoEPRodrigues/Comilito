# Video

> Status: being built on the `video` branch. Decision [0035](decisions/0035-video.md); design in
> `pencil/video-player.pen`.

Scenes can have videos. The top-most enabled scene with videos plays them in a window of its
own, meant for a TV or projector the players see. A video's own sound is never played. The
whole feature can be turned off in Preferences › Video.

## How it behaves

- **Which video shows:** the same rule as music. The top-most enabled scene with videos plays
  its playlist. A scene without videos leaves the video below showing, unless its *Hide the
  video of the scenes below* switch is on. Esc (stop all) blanks the window; Space pauses it.
- **Playlist:** in order or shuffled. *Switch videos every N minutes* plays a video for N
  minutes, then the next; *Coming back to a video* resumes where it stopped or starts it over.
  `0` minutes plays each video to its end. These follow the project's defaults unless the scene
  sets its own.
- **Video effects:** each scene can list videos for the video effects pad, under the sound
  effects. Playing one replaces the video until it ends; then the playlist comes back where it
  was.
- **Files:** videos are copied into the project's `video/` folder (with subfolders, like
  `audio/`), and appear in the Library's Video category.
- **Formats:** WebM (VP8, VP9, AV1) plays everywhere. Other videos (MP4/H.264, MOV, AVI...) may
  not play on Linux without extra GStreamer packages, so Comilito offers to convert them to WebM
  with its ffmpeg: when they're added, dropped, downloaded, or found in the `video/` folder.

## Looking at and converting videos

- Each video without `info` is looked at with ffprobe in the background (the ticker, one at a
  time). This needs the download tools (ffmpeg and ffprobe); without them videos stay "not
  looked at yet".
- Videos added in this session (Add files, dropped, found in `video/`, downloaded) that turn
  out not to play everywhere are collected, and once nothing is left to look at they're offered
  together in the Convert dialog (or converted, or left, per Preferences › Video).
- Conversions run one at a time with ffmpeg: VP9, constant quality (crf 32), no sound, same size.
  The working file is hidden (`.name.converting.webm`) so the folder scan ignores it, and is
  deleted if the conversion is cut off. The converted file replaces the original in every scene;
  the original goes to the trash, or stays as a video of its own ("(original)").
- Conversions aren't resumed after a restart; the video stays marked "may not play".

## Downloading videos

- After looking a page up, the URL dialog offers *Download as: Audio / Video*. Video downloads
  the picture only (yt-dlp `bv[height<=H][ext=webm]/bv[height<=H]/b[height<=H]`), up to the
  chosen resolution (Preferences › Video sets the default, 1080p).
- For each limit, the dialog shows the format that would be picked: WebM with VP9 or AV1 is
  preferred (green badge, plays here); when a site only has MP4, the video is converted after
  downloading (the usual Convert dialog, or automatically, per Preferences). "Best available"
  is the best format that plays here, or the best overall if none does.
- A direct link to a video file (`.webm`, `.mp4`...) is downloaded whole, as a video.
- "Only part of it", the queue, progress, cancelling, resuming after a restart and "Download
  again" (on the video's page) work as for audio.

## The video window

- A second Tauri window, labelled `video`, without a title bar; `main.ts` mounts
  `VideoWindow.svelte` in it instead of the app. Its top bar (shown while the pointer moves)
  is the drag handle; double-click or F toggles fullscreen; it has a screen picker and a close
  button. The screen and fullscreen are remembered in the preferences, and the window-state
  plugin remembers its size and place.
- Rust decides what should show: `TargetMix::video` (the playlist with its settings filled in),
  the transport and the playing video effect, all in the snapshot. The window decides the rest:
  the play order (shuffled per pass, never the same video twice in a row), switching after N
  minutes of play (paused time doesn't count), resuming, and the crossfades (two `<video>`
  layers; a third above them for video effects).
- Where each video stopped is sent to Rust every few seconds and when it's switched away, so
  closing and reopening the window resumes too. It's cleared when the project closes.
- The window reports what it shows about once a second (`video-status` event), for the bottom
  bar's now-playing.
- Closing the main window closes the video window (the app only exits when every window is
  closed).

## Build plan

Each step is committed and can be tried on its own.

1. [x] Data model and save format: `videos`, a scene's `video`, `hidesVideo` and
   `videoEffects`, `defaults.video`, `backups.includeVideo`, and the `video` preferences.
2. [x] Video files in the project: the `video/` folder, adding files, finding new or missing
   files, the Library's Video category and a video's page.
3. [x] Measuring videos with ffprobe, the conversion queue, and the Convert dialog (on import,
   on drop and for files found in the folder).
4. [x] The scene editor's Video section and video effects, and the drawer's Video column.
5. [x] The video window: a second window that plays the top scene's videos, follows Space and
   Esc, has the header button and the bottom bar's now-playing.
6. [x] Playlists: shuffle, switching every N minutes, resuming, crossfades.
7. [x] Video effects on the right column, sharing the space with the sound effects.
8. [x] The video downloader.
9. [ ] Preferences › Video, Project settings, export and backups with videos, and the on/off
   switch hiding everything.
