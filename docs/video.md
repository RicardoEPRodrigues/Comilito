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

## Build plan

Each step is committed and can be tried on its own.

1. [x] Data model and save format: `videos`, a scene's `video`, `hidesVideo` and
   `videoEffects`, `defaults.video`, `backups.includeVideo`, and the `video` preferences.
2. [x] Video files in the project: the `video/` folder, adding files, finding new or missing
   files, the Library's Video category and a video's page.
3. [ ] Measuring videos with ffprobe, the conversion queue, and the Convert dialog (on import,
   on drop and for files found in the folder).
4. [ ] The scene editor's Video section and video effects, and the drawer's Video column.
5. [ ] The video window: a second window that plays the top scene's videos, follows Space and
   Esc, has the header button and the bottom bar's now-playing.
6. [ ] Playlists: shuffle, switching every N minutes, resuming, crossfades.
7. [ ] Video effects on the right column, sharing the space with the sound effects.
8. [ ] The video downloader.
9. [ ] Preferences › Video, Project settings, export and backups with videos, and the on/off
   switch hiding everything.
