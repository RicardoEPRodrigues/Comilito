# Decision records

Short records of decisions made while planning and building Comilito, with the reasons behind them.
They're never rewritten: if a decision changes, a new record supersedes the old one.

## Index

| # | Decision | Status | Date |
|---|----------|--------|------|
| [0001](0001-tech-stack-tauri.md) | Use Tauri 2 with Rust and Svelte | Accepted | 2026-11-12 |
| [0002](0002-audio-in-rust.md) | Play all audio in Rust (kira), not the webview | Accepted | 2026-11-12 |
| [0003](0003-start-from-scratch.md) | Start from scratch instead of forking Ascapes Mixer | Accepted | 2026-11-12 |
| [0004](0004-save-format.md) | `.comilito` zip bundle with optional audio and relative paths | Superseded by 0006 | 2026-11-12 |
| [0005](0005-storage-location-and-hashing.md) | Default storage in Documents, zip as the only format, lazy hashing | Partly superseded by 0006 | 2026-11-12 |
| [0006](0006-project-folders.md) | A project is a folder; zip is only for export/import | Accepted | 2026-11-12 |
| [0007](0007-backups.md) | Automatic backups using the export zip | Accepted (location: 0026) | 2026-11-12 |
| [0008](0008-scenes.md) | Scenes: saved mixes with smart crossfades | Accepted | 2026-11-12 |
| [0009](0009-overlay-scenes-and-roles.md) | Overlay scenes with layer roles | Superseded by 0010 (no role: 0029) | 2026-11-12 |
| [0010](0010-scene-stack.md) | Scene stack instead of a single overlay | Amended by 0011 | 2026-11-12 |
| [0011](0011-stack-ui-and-presets.md) | Saved stack, presets, drag-and-drop stack UI | Accepted | 2026-11-12 |
| [0012](0012-preferences.md) | Preferences window with app and project levels | Accepted | 2026-11-12 |
| [0013](0013-plain-svelte-not-sveltekit.md) | Plain Svelte with Vite instead of SvelteKit | Accepted | 2026-11-12 |
| [0014](0014-html5-drag-and-drop.md) | Hand-written HTML5 drag and drop for the stack | Accepted (file drops on Linux: 0022) | 2026-11-13 |
| [0015](0015-loudness-normalization.md) | Loudness normalization with EBU R128 | Accepted | 2026-11-13 |
| [0016](0016-themes-and-desktop-accent.md) | Switchable themes, custom themes and the desktop accent colour | Accepted (theme files: 0025) | 2026-11-13 |
| [0017](0017-library-drawer-and-presets-tab.md) | Presets get their own tab; sounds become a collapsible Library drawer | Partly proposed | 2026-11-13 |
| [0018](0018-library-workspace.md) | A full-screen Library workspace for editing sounds | Accepted | 2026-11-13 |
| [0019](0019-download-audio-from-urls.md) | Download audio from video sites and direct URLs (no streaming) | Accepted (ffmpeg source superseded by 0051) | 2026-11-13 |
| [0020](0020-cropping-sounds.md) | Crop sounds in the app, saving a copy or overwriting | Accepted | 2026-11-13 |
| [0021](0021-no-play-edit-mode.md) | Remove the Play / Edit mode; everything is always editable | Accepted | 2026-11-13 |
| [0022](0022-dropping-files-onto-the-window.md) | Import audio by dropping files onto the window | Accepted (drop target: 0034) | 2026-11-13 |
| [0023](0023-home-screen-and-projects-folder.md) | Home screen, recent projects and moving the projects folder | Accepted | 2026-11-13 |
| [0024](0024-background-move-and-window-state.md) | Move the projects folder in the background; remember the window size | Accepted | 2026-11-13 |
| [0025](0025-theme-packages.md) | Themes can bring a watermark and fonts, shared as `.comilito-theme` packages | Accepted (editor: 0027) | 2026-11-13 |
| [0026](0026-backups-in-app-data.md) | Keep backups in the app's data folder, not the projects folder | Accepted (folder name: 0058) | 2026-11-13 |
| [0027](0027-theme-editor-files.md) | The theme editor can add a woodcut, font files and licences | Accepted | 2026-11-13 |
| [0028](0028-scene-volumes.md) | Each scene has its own music, ambience and effects volumes | Accepted | 2026-11-14 |
| [0029](0029-every-layer-has-a-role.md) | Every ambience layer has a role (Foreground by default) | Accepted (ducking everything: 0031) | 2026-11-14 |
| [0030](0030-os-media-controls.md) | Register with the OS as a media player | Accepted | 2026-11-14 |
| [0031](0031-scenes-that-duck-all-ambience.md) | A scene can duck all the ambience below | Accepted | 2026-11-14 |
| [0032](0032-choosing-the-audio-output.md) | Choosing the audio output | Accepted | 2026-11-14 |
| [0033](0033-resuming-downloads-and-quit-warning.md) | Resuming downloads after a restart, and warning when quitting while busy | Accepted | 2026-11-14 |
| [0034](0034-dropping-files-on-a-library-folder.md) | Dropping files on a Library folder imports them there | Accepted | 2026-11-15 |
| [0035](0035-video.md) | Videos for scenes, in a window of their own | Accepted (loading on Linux: 0037, then 0056; Space: 0038; kinds: 0039) | 2026-11-15 |
| [0036](0036-ui-testing-with-webdriver.md) | Test the running app with WebDriver, on the real Wayland desktop | Accepted | 2026-11-15 |
| [0037](0037-streaming-videos-on-linux.md) | Streaming videos into a MediaSource on Linux | Superseded by 0056 | 2026-11-15 |
| [0038](0038-video-keeps-playing-on-pause.md) | The video keeps playing while the sound is paused | Accepted | 2026-11-15 |
| [0039](0039-ambience-videos-and-video-effects.md) | Ambience videos and video effects are kept apart, with Library tabs | Accepted | 2026-11-15 |
| [0040](0040-crossfading-video-loops.md) | Videos crossfade back to their start unless marked as seamless loops | Accepted | 2026-11-15 |
| [0041](0041-gstreamer-in-the-appimage.md) | Bundle a chosen set of GStreamer plugins in the AppImage | Accepted | 2026-11-15 |
| [0042](0042-projects-outside-the-projects-folder.md) | Open projects from outside the projects folder, in place | Accepted | 2026-11-16 |
| [0043](0043-duplicating-sounds-and-videos.md) | Duplicating a sound or video copies its file | Accepted | 2026-11-16 |
| [0044](0044-restarting-the-interface.md) | Restart the interface by reloading the webviews | Accepted | 2026-11-16 |
| [0045](0045-video-window-screen-on-wayland.md) | Put the video window on its screen through fullscreen on Wayland | Accepted | 2026-11-16 |
| [0046](0046-packaging-linux-in-a-container.md) | Package Linux builds as an AppImage, in an Ubuntu 22.04 container | Accepted (Flatpak: 0059) | 2026-11-16 |
| [0047](0047-no-nvidia-video-decoding.md) | Decode videos without NVIDIA's hardware decoders on Linux | Superseded by 0054 | 2026-11-16 |
| [0048](0048-interface-scale.md) | Scale the interface with the webview's zoom | Accepted | 2026-11-18 |
| [0049](0049-mini-player.md) | A mini player in the main window, also used by narrow windows | Accepted | 2026-11-19 |
| [0050](0050-accent-following-themes.md) | Theme colours that follow the accent; Birch replaces Hearth | Accepted | 2026-11-19 |
| [0051](0051-gpu-video-encoding.md) | Convert videos on the GPU; ffmpeg from BtbN's release builds | Accepted (keyframes in 0055) | 2026-11-19 |
| [0052](0052-gpu-video-decoding-preference.md) | A preference to decode videos on NVIDIA GPUs (Linux) | Accepted (AppImage in 0053, default in 0054) | 2026-11-19 |
| [0054](0054-gpu-video-decoding-by-default.md) | Decode videos on NVIDIA GPUs by default (Linux) | Accepted | 2026-11-20 |
| [0055](0055-keyframes-for-gpu-conversions.md) | A keyframe every 60 frames in GPU conversions | Accepted | 2026-11-20 |
| [0056](0056-file-urls-for-videos-on-linux.md) | Play videos from `file://` URLs on Linux | Accepted | 2026-11-20 |
| [0057](0057-newer-nvidia-decoders.md) | Use nvcodec's newer decoders on GStreamer 1.20 | Accepted | 2026-11-20 |
| [0058](0058-backups-filed-by-project-id.md) | File backups by a project id, not only its name | Accepted | 2026-11-20 |
| [0059](0059-flatpak-on-flathub.md) | Distribute a Flatpak on Flathub, as `me.colorcrow.comilito` | Flathub superseded by 0063 (app ID and sandbox still apply; manifests: 0061) | 2026-11-20 |
| [0060](0060-website-on-github-pages.md) | A website in `website/`, published to GitHub Pages by a workflow | Accepted | 2026-11-20 |
| [0061](0061-flatpak-builds-and-releases.md) | Two Flatpak manifests (local and Flathub), and a script for releases | Accepted | 2026-11-20 |
| [0062](0062-gpl-3.0-licence.md) | License Comilito under the GPL-3.0 | Accepted | 2026-11-20 |
| [0063](0063-flatpak-on-github-releases.md) | Publish the Flatpak ourselves, with every package, on GitHub releases | Flatpak superseded by 0064 (GitHub releases still apply) | 2026-11-20 |
| [0064](0064-drop-flatpak-for-now.md) | Drop the Flatpak for now; the AppImage is Linux's package | Accepted | 2026-11-21 |
| [0065](0065-appimage-in-the-app-menu.md) | The AppImage adds itself to the app menu, always asking first | Accepted | 2026-11-21 |
| [0053](0053-nvidia-decoders-in-the-appimage.md) | Carry NVIDIA's GStreamer decoders in the AppImage | Accepted | 2026-11-19 |

## Template

```markdown
# NNNN. Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
What problem or question led to this decision.

## Options considered
- Option A: pros / cons
- Option B: pros / cons

## Decision
What we chose.

## Consequences
What this makes easier, harder, or commits us to.
```
