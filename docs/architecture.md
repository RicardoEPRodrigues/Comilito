# Architecture

> Status: planned. Updated as the code takes shape.

## Overview

```
┌─────────────────────────────────────────────┐
│  Webview (Svelte + TypeScript)              │
│  - Sound cards, sliders, buttons            │
│  - Holds UI state only                      │
└──────────────┬──────────────────────────────┘
               │ Tauri commands  (UI → Rust)
               │ Tauri events    (Rust → UI)
┌──────────────▼──────────────────────────────┐
│  Rust core (src-tauri)                      │
│  - Audio engine (kira)                      │
│  - Project model (categories, sounds)       │
│  - Save / load / import / export            │
└─────────────────────────────────────────────┘
```

The Rust side owns the project model and is the source of truth. The UI shows that state
and sends commands that change it.

## Audio engine

Built on [kira](https://github.com/tesselode/kira):

- One **kira mixer sub-track per category** (Music, Ambience, Effects), each with its own volume.
  A master volume sits above them.
- Every sound plays into its category's track, so a category slider affects all its sounds.
- Fades (fade-in, fade-out, crossfade) use kira tweens, so they're smooth and don't depend on UI timing.

Planned default behaviour per category (each can be overridden per sound):

| Category | Loop | Multiple at once | Typical fade |
|----------|------|------------------|--------------|
| Music    | yes  | no (crossfade to the new track) | 2–3 s |
| Ambience | yes  | yes (layers)     | 1–2 s |
| Effects  | no   | yes              | none |

Long files (music) are **streamed** from disk; short files (effects) are loaded into memory
so they start instantly.

## Command surface (draft)

| Command | Purpose |
|---------|---------|
| `play_sound(id)` / `stop_sound(id)` | Start or stop a sound (with its configured fade) |
| `set_sound_volume(id, v)` | Per-sound volume |
| `set_category_volume(category, v)` | Category track volume |
| `set_master_volume(v)` | Master volume |
| `stop_all()` | Panic button |
| `add_sounds(category, paths)` / `remove_sound(id)` / `update_sound(id, settings)` | Edit the library |
| `list_projects()` / `create_project(name)` / `open_project(folder)` | Project folders under `~/Documents/Comilito/` |
| `export_project(path, include_audio)` / `import_project(path)` | `.comilito` zip for moving projects |
| `relink_missing(folder)` / `relink_sound(id, path)` | Fix missing audio (files are copied into the project) |

Events from Rust to UI: `playback-state-changed`, `project-changed`, `missing-files`,
`hashing-progress`, `project-changed-on-disk` (for example, updated by a sync tool).

## Source layout (planned)

```
src/                 Svelte frontend
src-tauri/src/
  main.rs            Tauri setup, command registration
  audio.rs           kira wrapper: tracks, play/stop/fade
  project.rs         data model (serde structs)
  storage.rs         save/load, bundle zip, relinking
docs/
```
