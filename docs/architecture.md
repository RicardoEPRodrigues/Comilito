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
- Volumes are linear multipliers from 0.0 to 1.0 everywhere (UI, commands, `comilito.json`),
  so ducking can simply multiply them. They're converted to decibels only when handed to kira.
  Volume changes are smoothed over 50 ms so dragging a slider doesn't click.
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
| `get_volumes()` | Current master and category volumes |
| `stop_all()` | Panic button |
| `add_sounds(category, paths)` / `remove_sound(id)` / `update_sound(id, settings)` | Edit the library |
| `list_projects()` / `create_project(name)` / `open_project(folder)` | Project folders under `~/Documents/Comilito/` |
| `export_project(path, include_audio)` / `import_project(path)` | `.comilito` zip for moving projects |
| `relink_missing(folder)` / `relink_sound(id, path)` | Fix missing audio (files are copied into the project) |
| `get_preferences()` / `set_preferences(p)` | App preferences ([preferences.md](preferences.md)) |
| `set_project_defaults(d)` | Project-level default levels |

Temporary, for testing the engine before the project model exists:
`play_file(path, category)` plays a file by absolute path into a category's track. It will be removed once `play_sound(id)` works.

Events from Rust to UI: `playback-state-changed`, `project-changed`, `missing-files`,
`hashing-progress`, `project-changed-on-disk` (for example, updated by a sync tool).

## Source layout

```
index.html           Webview entry page
src/                 Svelte frontend (plain Svelte + Vite, see decision 0013)
  main.ts            Mounts App.svelte
  App.svelte         Root component
src-tauri/
  tauri.conf.json    App name, window, build settings
  capabilities/      Which Tauri APIs the window may use
  src/
    main.rs          Desktop entry point; just calls lib.rs
    lib.rs           Tauri setup, command registration
    audio.rs         kira wrapper: category tracks, volumes, play/stop
    project.rs       data model (serde structs); so far only `Category`
    storage.rs       (planned) save/load, bundle zip, relinking
docs/
```

The Rust code lives in `lib.rs` rather than `main.rs` so that the same code can later be
built for Android, where Tauri needs it as a library.
