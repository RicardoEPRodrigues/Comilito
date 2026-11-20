# Video

> Status: implemented. Decisions
> [0035](decisions/0035-video.md), [0037](decisions/0037-streaming-videos-on-linux.md),
> [0038](decisions/0038-video-keeps-playing-on-pause.md),
> [0039](decisions/0039-ambience-videos-and-video-effects.md) and
> [0040](decisions/0040-crossfading-video-loops.md); design in `pencil/video-player.pen`.

Scenes can have videos. The top-most enabled scene with videos plays them in a window of its
own, meant for a TV or projector the players see. A video's own sound is never played. The
whole feature can be turned off in Preferences › Video.

## How it behaves

- **Which video shows:** the same rule as music. The top-most enabled scene with videos plays
  its playlist. A scene without videos leaves the video below showing, unless its *Hide the
  video of the scenes below* switch is on. Esc (stop all) blanks the window. Space pauses the
  sound but the video keeps playing, unless that's turned off in Preferences › Video.
- **Playlist:** in order or shuffled. *Switch videos every N minutes* plays a video for N
  minutes, then the next; a video shorter than that loops until its N minutes are up (unless
  they end about when the video does). *Coming back to a video* resumes where it stopped or
  starts it over. `0` minutes plays each video to its end (the default is 10 minutes). These follow the
  project's defaults unless the scene sets its own.
- **Looping:** a playlist of one video loops it, and so does a video shorter than its turn (above). Unless the video is marked *Loops seamlessly*
  (on its page in the Library), the window starts it again on its second layer a moment before
  the end and crossfades into it, so the jump back doesn't show as a cut. Seamless videos (and
  ones too short for the fade) jump straight back.
- **Video effects:** each scene can list videos for the video effects pad, under the sound
  effects. Playing one replaces the video until it ends; then the playlist comes back where it
  was.
- **Two kinds**, like sound categories: *ambience videos* go in scenes' playlists, *video
  effects* on the video effects pad. A scene's sections each take only their kind.
- **Files:** videos are copied into the project's `video/ambience/` or `video/effects/` folder
  (with subfolders, like `audio/`), and appear in the Library's Videos tab (the Library has
  Sounds / Videos tabs, in the workspace and in the drawer). Changing a video's type moves it.
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
- Conversions run one at a time with ffmpeg, no sound, same size. On a GPU when one works
  ([decision 0051](decisions/0051-gpu-video-encoding.md)): AV1 through NVENC (constant quality
  36), VA-API, Quick Sync or AMF, with a keyframe every 60 frames so jumps into the video are
  quick ([0055](decisions/0055-keyframes-for-gpu-conversions.md)). Otherwise, or when the GPU fails (the CPU then redoes it), on the
  CPU: VP9, constant quality (crf 32). Preferences › Video › *Convert videos with* picks
  Automatic, a GPU, or CPU only. The GPUs are found the first time they're needed, by a tiny test
  encode with each candidate, and logged.
  The working file is hidden (`.name.converting.webm`) so the folder scan ignores it, and is
  deleted if the conversion is cut off. The converted file replaces the original in every scene;
  the original goes to the trash, or stays as a video of its own ("(original)").
- Unfinished conversions are kept in `conversions.json` in the app data folder: when the app
  quits or the project closes they stop, and they start again (from the beginning; ffmpeg can't
  continue a half-written file) the next time the project opens.

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
  button. Fullscreen goes to the chosen screen (through GTK on Linux, which Wayland allows); on
  Wayland a window that isn't fullscreen can't be moved by the app, so the window says to drag
  it there (decision 0045). Screens get "Screen 1", "Screen 2"... when the OS has no name for
  them (Wayland calls a laptop's screen "Unknown") or two share a name. Space works there as in
  the main window; Esc leaves fullscreen, or, when the window isn't fullscreen, stops everything
  as in the main window. The *restart the interface* key (F5) reloads just the video window;
  restarting from the main window reloads both. The screen and fullscreen are remembered in the
  preferences, and the window-state plugin remembers its size and place.
- Rust decides what should show: `TargetMix::video` (the playlist with its settings filled in),
  the transport and the playing video effect, all in the snapshot. The window decides the rest:
  the play order (shuffled per pass, never the same video twice in a row), switching after N
  minutes of play (paused time doesn't count), resuming, and the crossfades (two `<video>`
  layers; a third above them for video effects).
- Where each video stopped is sent to Rust every few seconds and when it's switched away, so
  closing and reopening the window resumes too. It's kept across restarts in
  `video-positions.json` in the app data folder (per project folder, not in the project, so a
  synced folder isn't re-uploaded every few seconds), written when the project closes and at
  most every 30 seconds meanwhile.
- The window reports what it shows about once a second (`video-status` event), for the bottom
  bar's now-playing.
- Closing the main window closes the video window (the app only exits when every window is
  closed).

## Loading videos into the webview

`src/lib/videoSource.ts` (`loadVideo`, and the `videoSource` action for the Library) sets the
`<video>`'s `src`. On Windows and macOS it's the file's asset URL. WebKitGTK's `<video>` loads
only http(s), blob and file URLs, so on Linux it's the file's `file://` URL, which GStreamer reads
straight from disk ([0056](decisions/0056-file-urls-for-videos-on-linux.md)). WebKit lets a page
show local files only when the page's scheme is registered as "local", which Rust does for each
window (`allow_local_media` in `video.rs`); the page still can't read them (`fetch`, canvas
pixels). Conversions put the WebM's seek index (Cues) at the front, so jumps don't read the end
of the file first.

On Linux, GStreamer uses NVIDIA's hardware video decoders when there are any
([0054](decisions/0054-gpu-video-decoding-by-default.md)). On GStreamer before 1.22, Comilito
makes it pick the newer ones (`nvvp9sldec`, `nvvp8sldec`) over the older `nvvp9dec`, which lagged
or stopped after jumps ([0057](decisions/0057-newer-nvidia-decoders.md)). With Preferences › Video › "Decode
videos on the GPU" off ([0052](decisions/0052-gpu-video-decoding-preference.md)), Comilito keeps
GStreamer from picking them (`GST_PLUGIN_FEATURE_RANK`, set at startup), as it did for everyone
when they froze after jumps ([0047](decisions/0047-no-nvidia-video-decoding.md)). The AppImage carries
NVIDIA's decoder plugin too, which uses the system's NVIDIA driver
([0053](decisions/0053-nvidia-decoders-in-the-appimage.md)).

## Trimming

A video's page has *Trim…*: choose the part to keep (typed, or taken from the preview's
position), preview it, then save it as a copy ("(trimmed)") or replace the original (which goes
to the trash and whose name is kept). The part is re-encoded to WebM in the conversion queue, so
the cut is exact to the frame and it runs in the background; it shows its progress, can be
cancelled, and resumes like any conversion. Re-encoding only the part makes short trims of long
videos quick.

## Known limits

The Windows and macOS side is untested (TODO.md).
