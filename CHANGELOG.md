# Changelog

All notable changes to Comilito are listed here, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

When a version is released, its changes move from **Unreleased** into a new section with the
version number and date.

## [Unreleased]

Nothing has been released yet; everything below is the first version, 0.1.0.

### Added

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
- **Themes:** Codex, Ironwood and Hearth, your own themes, and the desktop's accent colour.
- **Dropping audio files** from the file manager onto the window imports copies of them.
- **Scene groups** can be renamed and ungrouped from their ⋯ menu (or right-click), and scenes
  moved between groups.
- **Home screen:** continue with the last project, other projects ordered by when you last used
  them, duplicate a project, and an option to open the last project straight away at start.
- **Moving the projects folder** to another place, from Preferences, with a progress dialog.
- The window remembers its size, position and maximised state from the last time.
- Missing audio can be relinked; files added to a project's audio folders appear by themselves.
- Logs (Preferences › See logs) and a "Remove all data" option for uninstalling.

### Fixed

- Waveforms sometimes stayed blank until the sound was played, and jumped in size when the
  playback time appeared.
- Dropping a file on the window could replace the app with a media player playing that file.
- The window icon looked blurry in GNOME Shell.
- "Remove all data and quit" left the window-size file behind.
- When Comilito is started without a terminal (for example by double-clicking the AppImage), its
  error messages and crashes are now written to `stderr.log` and `comilito.log` in the logs
  folder instead of being lost.

[Unreleased]: https://github.com/RicardoEPRodrigues/Comilito/commits/main
