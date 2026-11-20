# Changelog

All notable changes to Comilito are listed here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

When a version is released, its changes move from **Unreleased** into a new section with the
version number and date.

## Unreleased

Nothing has been released yet; everything below goes into the first release, 0.2.0.

### Added

- **Videos decode on the GPU** (Linux, NVIDIA): NVIDIA's GStreamer decoders play videos again,
  taking work off the CPU, in the AppImage too. On older GStreamer (Ubuntu 22.04, the AppImage)
  Comilito picks NVIDIA's newer decoders, which don't lag or stop after a jump. A switch in Preferences › Video turns this off
  (it applies the next time Comilito starts) if videos freeze after a jump. (Intel and AMD GPUs,
  and Windows and macOS, already decoded on the GPU.)
- **Videos convert on the GPU** when it can: NVIDIA (RTX 40 and newer), recent Intel and AMD
  GPUs encode AV1, which plays everywhere, many times faster than the CPU (a 30-second 1080p60
  clip: about 3 s instead of 47 s on an RTX 5070 Ti). Preferences › Video › *Convert videos with*
  picks Automatic, a GPU (with two, choose which) or CPU only. If the GPU fails, the CPU takes
  over. The conversion list says when a video is being converted on the GPU. Their keyframes
  come every 60 frames, so a scene that resumes a video partway in shows it moving within a
  second on Linux too, where AV1 decodes on the CPU.

- **Projects:** one folder per campaign (`comilito.json` plus its `audio/` folder), saved
  automatically, with automatic backups, export and import as `.comilito` files (with or without
  audio), and a sample project to try things out.
- **Sounds** in three categories (Music, Ambience, Effects), each with its own volume, loop and
  fades, plus loudness normalization so quiet and loud files play at a similar level.
- **Scenes and the Tapestry:** scenes combine music, ambience layers and effects; stacking them
  in the Tapestry ducks the ambience below and plays the top scene's music. Scene hotkeys,
  Space to pause, Esc to stop.
- **Presets** to save and reload whole Tapestries, in groups.
- **Library** with folders, tags, favourites, search, smart lists (recent, unused, missing) and
  a full-screen workspace for editing sounds, several at once if needed.
- **Downloads** from YouTube and other sites (through yt-dlp, which Comilito downloads and
  updates itself) or from direct audio links, optionally only part of a long video.
- **Cropping** sounds on their waveform, saving a copy or replacing the original.
- **Themes:** Codex (parchment), Ironwood (dark) and Birch (clear and light), your own themes,
  and the desktop's accent colour. Codex's header follows the accent you choose. A theme can
  colour its header, side panel or mixer bar with whatever accent is in use, or keep its own
  accent whatever is chosen (for themes built around one colour).
- **Hearth** is now an optional theme in `extras/themes/` (import it from Preferences ›
  Appearance); its red side panel follows the accent.
- **Themes can bring their own woodcut and fonts.** Themes are now shared as `.comilito-theme`
  files, which can include a watermark (drawn in the theme's gold, in place of the skeletal
  horse), font files and their licences. Theme files exported earlier (`.json`) still import.
- Two optional, unbundled themes in `extras/themes/`: **Horde** and **Alliance**.
- The **theme editor** can give a theme its own woodcut (painted in the theme's gold), font
  files and their licences, all previewed live before saving.
- **Library folders can be moved:** drag a folder onto another folder (or onto its category), or
  use *Move to* in its right-click menu. The sounds inside follow.
- *Add from URL here…* in a Library folder's (or category's) right-click menu downloads straight
  into that folder.
- **Each scene has its own Music, Ambience and Effects volumes.** The bottom bar's sliders
  now set the volumes of the top scene in the Tapestry (its name is shown above them), and
  the volumes fade along when another scene comes on top. Master stays the same for the whole
  project. Scenes you haven't adjusted use the project's volumes.
- **Backspace removes the top scene** from the Tapestry. The key can be changed (or turned off)
  in Preferences › Behaviour › Shortcuts.
- **Middle-click** a scene in the Tapestry to disable it, and again to enable it.
- The **Library drawer can be resized** by dragging its top edge; double-click the edge to go
  back to the default size. The size is remembered.
- **Choose the audio output** in Preferences › Playback (for example the table's speakers while
  your headset is on a call). By default Comilito follows the desktop's default output.
- **Media keys and the desktop's media controls** play, pause and stop the Tapestry: Comilito
  registers with the OS as a media player, showing the top scene and the music playing.
- **Videos for scenes**, shown in a window of their own for a TV or projector (their sound is
  never played). Scenes get a video playlist: shuffled or in order, switching every few minutes
  and resuming each video where it left off. The top-most scene with videos shows them, like
  music; a scene can hide the video below. Video effects (a lightning strike, a reveal) play
  over the playlist until they end, from a pad under the sound effects. The video window has no
  title bar (drag its top bar), goes fullscreen on the screen you choose, and the header's Video
  button opens it. Videos live in the project's `video/` folder and the Library's Videos tab;
  videos that may not play (MP4 and others on Linux) are offered for conversion to WebM.
  Like sounds, videos come in two kinds, *ambience videos* and *video effects*, each going in
  its own place in scenes; the Library has Sounds and Videos tabs (in the workspace and in the
  drawer). Videos can be downloaded from URLs too (the picture only, up to a chosen resolution), and
  exported or backed up with the project if you want. Videos can be trimmed (exactly, in the
  background), remember where they left off across restarts, are fingerprinted like sounds,
  and unfinished conversions carry on when the project opens again. Space pauses only the sound (the video
  keeps playing; Esc blanks it). Everything video can be turned off in Preferences › Video.
  Playlists switch videos every 10 minutes unless set otherwise.
  A video playing on its own loops with a crossfade back to its start, so the jump doesn't show
  as a cut; videos made to loop can be marked *Loops seamlessly* in the Library to loop
  without it.
- **Loudness normalization can be turned on or off for one sound**, in its Loudness section in
  the Library, overriding its category's setting in Project settings.
- **Downloads continue after a restart.** Downloads still queued or running when Comilito quits
  resume, from where they stopped, the next time their project is opened.
- **A warning when quitting while busy:** closing Comilito during downloads, fingerprinting, a
  backup or a move of the projects folder asks first, listing what would be interrupted. It can
  be turned off in Preferences › Behaviour.
- **Fold or unfold all folders** with the two new buttons in the Library drawer's toolbar, or those of one
  category at the top of its column.
- **Smoother opening and closing:** the Library drawer slides open and shut, menus and dialogs
  ease in, and groups, folders and Tapestry entries unfold instead of jumping. All of it is
  turned off when the desktop asks for reduced motion.
- **Projects outside the projects folder:** *Open a project folder…* (in the home screen's More
  menu) opens a project wherever it is, on another drive or a synced folder, without copying
  it. It's listed on the home screen from then on, with the folder it's in; *Remove from list*
  forgets it without touching its files.
- **Duplicate a sound or a video** in the Library: *Duplicate* in a sound's right-click menu,
  on its page, and on a video's page. The copy gets its own file ("… (copy)") with the same
  settings, ready to be changed without touching the original, and isn't in any scene.
- **Restart the interface** with F5 (changeable in Preferences › Behaviour › Shortcuts), or the
  button in Preferences › Projects & tools › Troubleshooting and in About. It reloads the
  windows as if Comilito had just started, for when something on screen gets stuck; the project
  stays open and the sound keeps playing.
- **Linux download:** an AppImage that runs on Ubuntu 22.04 and newer (and other distros of the
  same age or newer). A Flatpak is planned; there's no `.deb` or `.rpm` package.
- **Scale the whole interface** (text, icons and spacing) from 50% to 200% in Preferences ›
  Appearance › Interface size, or with Ctrl + / Ctrl − (Ctrl 0 goes back to 100%). Remembered
  on this computer; the video window isn't scaled.
- **Mini player:** the header's new button shrinks the window to a small player that stays on
  top of other windows (the pin turns that off), for running a session next to a virtual
  tabletop or your notes. It shows the Tapestry, with scenes and presets in a drawer below
  (a click adds a scene or loads a preset), the effects pad on a second tab, play / stop, the
  master volume and the top scene's volumes, and the Video button. The arrows button goes back
  to the full window, at its old size and place. Quitting in the mini player starts in it again.

### Changed

- **Videos on Linux** play straight from their files, as on Windows and macOS, instead of being
  read in pieces by the window. Switching scenes shows the next video sooner, and the Library's
  thumbnails load faster.
- Comilito's ffmpeg now comes from BtbN's release builds (ffmpeg 8.1) instead of yt-dlp's nightly
  ones, whose GPU encoders need the very newest NVIDIA drivers. *Update* in Preferences ›
  Projects & tools › Download tools replaces an ffmpeg downloaded earlier.

- Comilito now presents itself as a soundboard for game masters of any tabletop role-playing game
  (Dungeons & Dragons among them), on the home screen and in About, which also credits
  dnd-soundboard as an inspiration.

- In an expanded Tapestry entry, a ducked ambience layer shows a ⌄ icon after its volume instead
  of "ducked from 60%"; hover it for the level it came down from.
- **Scenes say what they hold with icons:** in the Scenes tab, the Tapestry and the mini player,
  a scene's music tracks, ambience layers, effects and videos show as the Library's category
  icons with a count (and the Tapestry's ducking as "⌄ 30%"), instead of a sentence. Hover an
  icon, or the scene, for the words.

- **Narrow windows:** making the window narrower than about 860 px turns on the mini player
  (without keeping it on top; its pin does that), and widening it past that turns it off. The
  window can now be made as small as 360 × 420. Below 1100 px the side columns narrow, the
  Tapestry's entries get more compact, and the bottom bar drops its keyboard hint and shares
  its room between the sliders and the "now showing" video, instead of overlapping.

- **Backups** are kept in Comilito's data folder (`~/.local/share/com.comilito.app/backups` on
  Linux) instead of a `Backups` folder among your projects, so the projects folder holds only
  projects. Existing backups are moved there automatically. *Remove all data and quit* asks
  whether to keep them.
- **Dropping audio files** from the file manager onto the window imports copies of them. Dropped
  on a Library folder or category (in the drawer or the Library workspace), they go there.
- **Scene groups** can be renamed and ungrouped from their ⋯ menu (or right-click), and scenes
  moved between groups.
- **Home screen:** continue with the last project, other projects ordered by when you last used
  them, duplicate a project, and an option to open the last project straight away at start.
- **Moving the projects folder** to another place, from Preferences, with a progress dialog.
- The window remembers its size, position and maximised state from the last time.
- Missing audio can be relinked; files added to a project's audio folders appear by themselves.
- Logs (Preferences › See logs) and a "Remove all data" option for uninstalling.
- **Ambience roles:** every layer now has a role, and new layers start as Foreground; "No role"
  is gone (layers that had none become Foreground). The role can be changed right in the
  Tapestry, next to each layer of an expanded scene. A scene with no ambience of its own no
  longer ducks the ambience below it, unless its new *Duck all the ambience below* switch (in
  the scene editor) is on.
- The app icon is the plain horse-skull mark at every size; the decorated version was too busy.

### Fixed

- *Update* in Preferences › Projects & tools › Download tools shows what it's doing (checking,
  downloading with progress, unpacking) and then what was updated, or that everything was up to
  date. It no longer downloads yt-dlp again when it's already the newest.

- In Preferences › Appearance, a theme's name wraps onto a second line when it's long or its font
  is wide, instead of pushing the "Custom" badge out of its card.

- A sound's page in the Library has its Duplicate and Remove buttons next to its name, as icons,
  like a video's page; on both, Remove is red.

- A long effect name wraps inside its button on the effects pad instead of running out of it.
- The bottom bar's master volume lines up with the scene's volumes.
- In an expanded Tapestry entry, every sound's remove button lines up at the right edge (ambience
  rows used to push theirs further right), and the rows fit a narrow Tapestry.
- The mini player's button now always keeps the window on top: the window's own resize could
  briefly turn the mini player off and back on as if the window had been narrowed by hand.

- The bottom bar's "now showing" video no longer lets its text spill out of its box; long names
  and details end with "…".
- Dialogs fit small windows: they scroll when taller than the window, and "Save the current
  tapestry?" puts its fields and buttons one per line in a narrow window.

- On Windows, opening the video window froze the app: the window stayed white and the main
  window stopped responding.
- With *Switch videos every N minutes* on, a video shorter than N minutes moved on to the next
  one when it ended; it now loops until its N minutes are up.
- With the Library drawer closed, results of its quick search couldn't be dragged onto a scene
  or the tapestry: the pop-up closed as soon as a result was pressed.
- Videos often failed to load on Linux ("Media failed to decode") when starting partway in:
  the Library's thumbnails stayed empty and the video window showed nothing, while the video's
  own page still played it from the start. Seeking within long videos could fail the same way.
- On Linux computers with an NVIDIA graphics card, a video resumed partway in (switching scenes
  and back, thumbnails) could freeze on its first frame or stay black for several seconds before
  starting over. Comilito now decodes videos without NVIDIA's hardware decoder.
- Choosing a screen for the video window didn't move it on Wayland. Fullscreen now goes to the
  chosen screen; a window that isn't fullscreen can't be moved by apps there, so the window says
  to drag it (or press F). Screens without a proper name get "Screen 1", "Screen 2"... and the
  picker shows each screen's resolution.
- The video window's screen picker was hard to read (dark text on a dark list).
- A video stayed black in the video window, even after restarting Comilito, when it had last
  stopped in its final second (common for short videos that loop seamlessly). Positions in the
  last second now count as the start, and a video that doesn't load within 8 seconds is
  started from the beginning (and logged).
- The video window could freeze on a picture: a video starting to load less than a second after
  a crossfade could be thrown away by the layer clean-up of the crossfade before it. A video (or
  video effect) that stops moving is now started again where it was, and video problems are
  written to the log (*See logs*).
- The video's playing time in the video window's top bar didn't move.
- Esc in the video window now stops everything, as in the main window, when the window isn't
  fullscreen (in fullscreen it still leaves fullscreen first).
- *See error log* and *Open logs folder* next to *See logs* (Preferences › Projects & tools,
  and About), for the error output GTK and WebKit print, crashes included.
- The AppImage froze when opening a project with videos: it carried GStreamer without its
  plugins, so WebKit couldn't play any video. It now bundles the plugins needed for WebM, and
  keeps its own GStreamer plugin cache instead of overwriting the one other programs share.
  A video that can't be read no longer gets read whole in the background.
- After opening a finished video download (or a video from the drawer's search, or dropped
  files), clicking folders in the Library did nothing until Comilito was restarted.
- In the Library drawer, the scrollbar could cover the number of items next to a folder.
- A scene's details in the Tapestry now list its videos and video effects too, with a ✕ to
  take one out of the scene.
- Right-clicking most of the window opened the webview's own menu, with *Reload* and other
  browser actions. Only text fields keep it now (for cut, copy and paste).
- Downloading part of a video now shows its progress in percent, instead of only
  "downloading the selected part…".
- With an audio output chosen in Preferences, every sound started 1 to 2 seconds late.
- The bottom bar's Music, Ambience and Effects sliders jumped when another scene came on top;
  they now glide to the new volumes in step with the sound.
- A sound previewed in an editor kept playing after leaving it. Previews now stop when their
  editor closes, and the bottom bar shows what's being previewed with a button to stop it.
- In the Library drawer, folding a folder now hides its subfolders too.
- Scenes can be dragged into a group: drop one on the group's heading or on a scene in it (or
  on an ungrouped scene to take it out of its group).
- The start and end times for downloading part of a video are typed as `00:00:00` in one field
  each, and no longer overflow the dialog.
- Comilito no longer crashes at start on systems without a Documents folder set up; it uses
  `~/Documents` then.
- Titles in wide display fonts (for example "Comilito" on the home screen in the Alliance theme)
  no longer overflow their column; they shrink to fit.
- Waveforms sometimes stayed blank until the sound was played, and jumped in size when the
  playback time appeared.
- Dropping a file on the window could replace the app with a media player playing that file.
- The window icon looked blurry in GNOME Shell.
- "Remove all data and quit" left the window-size file behind.
- When Comilito is started without a terminal (for example by double-clicking the AppImage), its
  error messages and crashes are now written to `stderr.log` and `comilito.log` in the logs
  folder instead of being lost.
