# Architecture

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
  A master volume sits above them. The category volumes are those of the top-most enabled
  scene (or the project's, when it has none or no scene is enabled), and fade with the scene
  transition when the top scene changes ([0028](decisions/0028-scene-volumes.md)).
- Every sound plays into its category's track, so a category slider affects all its sounds.
- Volumes are linear multipliers from 0.0 to 1.0 everywhere (UI, commands, `comilito.json`),
  so ducking can simply multiply them. They're converted to decibels only when handed to kira.
  Volume changes are smoothed over 50 ms so dragging a slider doesn't click.
- The output is the system default, or one chosen in Preferences
  ([0032](decisions/0032-choosing-the-audio-output.md)). Changing it rebuilds the engine.
- Fades (fade-in, fade-out, crossfade) use kira tweens, so they're smooth and don't depend on UI timing.

Default behaviour per category (each can be overridden per sound; the fade times are the
defaults for new projects, see [preferences.md](preferences.md)):

| Category | Loop | Multiple at once | Typical fade |
|----------|------|------------------|--------------|
| Music    | yes  | no (crossfade to the new track) | 2 s |
| Ambience | yes  | yes (layers)     | 1.5 s |
| Effects  | no   | yes              | none |

Long files (music) are **streamed** from disk; short files (effects) are loaded into memory
so they start instantly.

## How state reaches the UI

Rust holds everything in one `AppState` behind one `Mutex` (`state.rs`): the audio engine, the
app preferences and the open project. Every command locks it, makes its change, recomputes the
**target mix** (`mix.rs`) and fades the engine to it, then emits **`state-changed`** with a full
`Snapshot` (project, target mix, playback info, missing files). The UI keeps the latest snapshot
in `src/lib/store.svelte.ts` and renders from it; it never works out changes itself.

Commands that change something return nothing on success (the new state comes as an event) and
reject with a message on failure. Problems that aren't a command's direct failure (a missing
audio file found while fading to a mix) arrive as **`notice`** events.

A background thread (`ticker` in `lib.rs`) runs four times a second: it advances music
playlists, saves `comilito.json` one second after the last change, notices when another program
changed the file (`changedOnDisk` in the snapshot), and emits `state-changed` when what's audible
changed. Every two seconds it also **rescans the project's `audio/` folders**
(`OpenProject::refresh_from_disk`): audio files that appeared there (copied in with a file
manager, or by a sync tool) become new sounds, in the category of their folder, once their size
is the same on two scans in a row; sounds whose file disappeared are marked missing (not
deleted). Polling was chosen over a file-watcher crate: three folders are cheap to scan, it
behaves the same on every OS, and it needs no dependency. It also does the slow jobs, one at a time and **with the state unlocked** so the UI
never waits: a backup every *N* minutes (when something changed), and hashing audio files that
don't have a `sha256` yet (`hashingRemaining` in the snapshot). Before those, it looks at new
videos with ffprobe (`VideoInfo`), and once none is left, offers the new ones that may not play
for conversion (`video_convert.rs`). Before hashing it also measures
**loudness** of files that haven't been measured (`analyzingRemaining`), then re-applies the mix
so the normalization gain takes effect.

## Command surface

All in `src-tauri/src/commands.rs`, with a typed wrapper per command in `src/lib/api.ts`.
Stack indexes count from the bottom (0 = bottom), like `stack` in `comilito.json`.

| Area | Commands |
|------|----------|
| General | `app_version`, `confirm_quit` (quit after the "still busy" warning), `get_state`, `get_preferences`, `set_preferences(prefs)` (switches the audio output if it changed), `list_output_devices`, `log_message(level, message)`, `open_log_file(which)` (`events`: comilito.log, `errors`: stderr.log, or `folder`) |
| Projects | `list_projects` (with counts and last edit), `projects_folder`, `duplicate_project(path)`, `move_projects_folder(to)`, `show_projects_folder(project?)`, `create_project(name, sample)`, `open_project(path)`, `delete_project(path, delete_audio)`, `close_project`, `reload_project`, `keep_my_version`, `rename_project(name)` |
| Moving projects | `export_project(dest, include_audio)`, `import_project(path)`, `backups_folder`, `relink_from_folder(folder)`, `relink_sound(id, path)` |
| Project settings | `set_master_volume(volume)`, `set_category_volume(category, volume)` (the top scene's, or the project's when no scene is enabled), `set_project_defaults(defaults)`, `set_backup_settings(backups)`, `reset_project_defaults` |
| Sounds | `add_sounds(category, folder, paths)` → new ids, `import_dropped_file` (raw file bytes; name, category and folder in headers), `update_sound(sound)`, `set_sound_category(id, category)`, `remove_sound(id, permanently)`, `preview_sound(id)`, `toggle_effect(id)` |
| Scenes | `create_scene(name, group)`, `update_scene(scene)`, `duplicate_scene(id)`, `delete_scene(id)`, `scene_set_group(scene_id, group)`, `scene_group_rename(from, to)`, `scene_group_ungroup(name)`, `set_layer_volume(scene_id, layer_index, volume)`, `set_layer_role(scene_id, layer_index, role)`, `add_sound_to_scene(scene_id, sound_id)`, `remove_sound_from_scene(scene_id, sound_id)` (a video id works too), `remove_missing_sounds` |
| Stack | `stack_insert(scene_id, index, mode)`, `stack_move(entry_id, index)`, `stack_remove(entry_id)`, `stack_set_enabled(entry_id, enabled)`, `stack_switch(scene_id)`, `scene_hotkey(scene_id)` |
| Presets | `preset_save(name)`, `preset_load(preset_id)`, `preset_insert(preset_id, index)`, `preset_rename(preset_id, name)`, `preset_delete(preset_id)` |
| Videos ([0035](decisions/0035-video.md), in `video.rs`) | `add_videos(kind, folder, paths)` → new ids, `update_videos(videos)` (name, tags, favourite, seamless loop), `remove_video(id, permanently)`, `move_videos(ids, kind, folder)` (a new kind moves the file and leaves scene slots of the other kind), `reveal_video(id)`, `relink_video(id, path)`, `add_video_to_scene(scene_id, video_id)`, `video_streaming` (whether the frontend streams videos into a MediaSource, Linux, [0037](decisions/0037-streaming-videos-on-linux.md)). Library folder commands take a *shelf*: an audio category, `"ambienceVideo"` or `"videoEffect"`. Converting (`video_convert.rs`): `convert_videos(ids, keep_originals)`, `trim_video(id, start_ms, end_ms, as_copy)` (a re-encoded part, in the same queue), `cancel_conversion(id)`, `clear_conversions`, `list_conversions`. The video window (`video_window.rs`): `toggle_video_window`, `show_video_window`, `list_screens`, `set_video_screen(name)`, `set_video_fullscreen(on)`, `play_video_effect(id)`, `video_effect_ended(id)`, `save_video_position(id, position_ms)`, `video_positions` |
| Transport | `toggle_play`, `stop_all` |
| Appearance | `desktop_appearance`, `list_user_themes`, `save_user_theme(theme)`, `delete_user_theme(id)`, `import_theme(path)` (a `.comilito-theme` package or a plain `.json`), `export_theme(theme, dest)` (writes a package), `read_theme_asset(name)` → raw bytes of a font or watermark a theme brought, `add_theme_file(path, role)` → stored name of a file picked in the theme editor ([themes.md](themes.md)) |

Events from Rust to UI: `state-changed` (a `Snapshot`), `notice` (a message string),
`projects-move-progress` and `projects-moved` (moving the projects folder, decision 0024),
`quit-requested` (a list of what's still running; see below), `conversions-changed` and
`convert-offer` (videos to convert, decision 0035), `preferences-changed` (so the video window
follows its settings), plus the download and tools events. The video window itself sends
`video-status` (what it shows) to the main window.

### Quitting

Closing the main window runs `on_window_event` in `lib.rs`. If downloads, fingerprinting, a
backup or a projects-folder move are running (`AppState::busy_work`) and the *Warn when quitting*
preference is on, the close is held back and `quit-requested` is emitted; the UI asks, and
*Quit anyway* calls `confirm_quit`, which sets `AppState::quitting` and closes the window again.
On exit, a running yt-dlp and any ffmpeg it started are stopped (`tools::kill_tree`; a child
process outlives its parent) and the project is
saved. Unfinished downloads are kept in `downloads.json` in the app data folder and resume when
their project is next opened ([0033](decisions/0033-resuming-downloads-and-quit-warning.md)).

## Transport

- **Stopped:** nothing from the stack plays. Opening a project starts here, so the saved stack is
  restored silently; **Space** starts it.
- **Playing → Paused (Space):** the Music and Ambience tracks are paused (with a short fade).
  kira stops processing a paused track's sounds, so positions and fades freeze and continue on
  resume. Stack changes while paused are applied but stay silent until resume.
- **Esc (stop all):** every sound fades out and positions are forgotten; the stack is kept.
- Effects and previews ignore pause.
- The OS's media controls (media keys, headset buttons, the desktop's player widget) can play,
  pause and stop too: Comilito registers as a media player (`media_controls.rs`, decision 0030)
  and shows the top scene, the music track and the project there.

## Playback details

- **Music:** each stack entry with music gets its own player. Covered or disabled music pauses
  and resumes where it left off; an entry removed from the stack stops its music. A playlist
  with one track loops it (if the sound loops); a longer playlist plays through and wraps around.
- **Ambience:** one player per sound. Sounds staying in the mix only change volume.
- **Effects:** loaded into memory (cached) so they start instantly. A pad button is a toggle:
  clicking an effect that's still playing stops it (so a looping effect is an on/off switch).
- **Previews** (▶ in the library) play straight into the main track, so they're heard even while paused.
  Only one plays at a time. It stops when the editor it was started from closes (a sound or
  crop editor, the scene editor, the Library workspace), and the mixer bar shows the previewed
  sound with a button to stop it from anywhere.
- Fades on scene changes use the scene's `transitionMs` (or the sound's own fade-in, if longer).
- **Loudness normalization** ([0015](decisions/0015-loudness-normalization.md)): every sound's
  playback volume is its linear volume plus a gain in dB that brings it to the project's target
  loudness (`Project::normalization_gain_db`). The mix itself (ducking, volumes) is unaware of it.

## Source layout

```
index.html           Webview entry page
src/                 Svelte frontend (plain Svelte + Vite, see decision 0013)
  main.ts            Mounts App.svelte
  App.svelte         Root component: project picker or workspace, global shortcuts
  app.css            Theme colours (light/dark) and base element styles
  lib/               types.ts (mirrors the Rust structs), api.ts (one function per command),
                     store.svelte.ts (latest snapshot + UI state), actions.ts (actions that ask first)
  components/        One Svelte component per panel, editor or dialog
src-tauri/
  tauri.conf.json    App name, window, build settings
  capabilities/      Which Tauri APIs the window may use
  src/
    main.rs          Desktop entry point; just calls lib.rs
    lib.rs           Tauri setup, command registration, background ticker thread
    commands.rs      Every Tauri command
    state.rs         AppState (shared behind a Mutex) and the Snapshot sent to the UI
    audio.rs         kira engine: category tracks, fade to a mix, playlists, effects, transport
    mix.rs           Pure stack -> target mix calculation (unit tested)
    project.rs       Data model (serde structs mirroring comilito.json)
    storage.rs       Project folders: list/create/load/save, copying audio in
    prefs.rs         App preferences (preferences.json)
    bundle.rs        .comilito zips (export, import, backups), sha256, finding missing files
    sample.rs        Builds the sample project from the bundled audio
    loudness.rs      Measures a file's EBU R128 loudness and peak
    media_controls.rs  Registers with the OS as a media player (MPRIS, Windows, macOS)
  resources/sample/  Sample audio, shipped with the app (`bundle.resources`), see CREDITS.md
docs/
```

The Rust code lives in `lib.rs` rather than `main.rs` so that the same code can later be
built for Android, where Tauri needs it as a library.
