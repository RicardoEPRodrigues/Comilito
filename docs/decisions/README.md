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
| [0019](0019-download-audio-from-urls.md) | Download audio from video sites and direct URLs (no streaming) | Accepted | 2026-11-13 |
| [0020](0020-cropping-sounds.md) | Crop sounds in the app, saving a copy or overwriting | Accepted | 2026-11-13 |
| [0021](0021-no-play-edit-mode.md) | Remove the Play / Edit mode; everything is always editable | Accepted | 2026-11-13 |
| [0022](0022-dropping-files-onto-the-window.md) | Import audio by dropping files onto the window | Accepted (drop target: 0034) | 2026-11-13 |
| [0023](0023-home-screen-and-projects-folder.md) | Home screen, recent projects and moving the projects folder | Accepted | 2026-11-13 |
| [0024](0024-background-move-and-window-state.md) | Move the projects folder in the background; remember the window size | Accepted | 2026-11-13 |
| [0025](0025-theme-packages.md) | Themes can bring a watermark and fonts, shared as `.comilito-theme` packages | Accepted (editor: 0027) | 2026-11-13 |
| [0026](0026-backups-in-app-data.md) | Keep backups in the app's data folder, not the projects folder | Accepted | 2026-11-13 |
| [0027](0027-theme-editor-files.md) | The theme editor can add a woodcut, font files and licences | Accepted | 2026-11-13 |
| [0028](0028-scene-volumes.md) | Each scene has its own music, ambience and effects volumes | Accepted | 2026-11-14 |
| [0029](0029-every-layer-has-a-role.md) | Every ambience layer has a role (Foreground by default) | Accepted (ducking everything: 0031) | 2026-11-14 |
| [0030](0030-os-media-controls.md) | Register with the OS as a media player | Accepted | 2026-11-14 |
| [0031](0031-scenes-that-duck-all-ambience.md) | A scene can duck all the ambience below | Accepted | 2026-11-14 |
| [0032](0032-choosing-the-audio-output.md) | Choosing the audio output | Accepted | 2026-11-14 |
| [0033](0033-resuming-downloads-and-quit-warning.md) | Resuming downloads after a restart, and warning when quitting while busy | Accepted | 2026-11-14 |
| [0034](0034-dropping-files-on-a-library-folder.md) | Dropping files on a Library folder imports them there | Accepted | 2026-11-15 |

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
