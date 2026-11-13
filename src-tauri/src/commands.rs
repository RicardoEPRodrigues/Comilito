//! Every Tauri command the UI can call. The list is documented in docs/architecture.md.
//!
//! Each command follows the same pattern:
//! 1. lock the shared `AppState`,
//! 2. change the project or the engine,
//! 3. call `project_changed` (recompute the mix, fade to it, mark for saving),
//! 4. `emit_state` so the UI shows the result.
//!
//! Commands that change something return `Result<(), String>`: on `Err`, the JavaScript
//! `invoke()` promise rejects with that message. The new state arrives via the
//! `state-changed` event, not as a return value, so there's one way state reaches the UI.

use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use tauri::{AppHandle, State};

use crate::{
    prefs::Preferences,
    project::{
        BackupSettings, Category, PresetEntry, ProjectDefaults, Scene, Sound, StackEntry,
        StackPreset, new_id,
    },
    state::{AppState, Snapshot, emit_state},
    storage::{self, ProjectSummary},
};

/// The type every command receives to reach the shared state.
/// `'_` is an anonymous *lifetime*: it says "this borrows something Tauri owns, for as long
/// as the command runs". You mostly see lifetimes when a type holds a reference.
type AppStateRef<'a> = State<'a, Mutex<AppState>>;

/// Locks the state. `lock()` fails only if a thread panicked while holding the lock.
/// The guard unlocks automatically when it goes out of scope (like C++ RAII).
fn lock<'a>(state: &'a AppStateRef) -> Result<MutexGuard<'a, AppState>, String> {
    state.lock().map_err(|_| "internal error: state lock poisoned".to_string())
}

fn millis(ms: u32) -> Duration {
    Duration::from_millis(ms as u64)
}

// ---- General --------------------------------------------------------------------------------

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn get_state(state: AppStateRef) -> Result<Snapshot, String> {
    Ok(lock(&state)?.snapshot())
}

#[tauri::command]
pub fn get_preferences(state: AppStateRef) -> Result<Preferences, String> {
    Ok(lock(&state)?.prefs.clone())
}

#[tauri::command]
pub fn set_preferences(prefs: Preferences, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.prefs = prefs;
    s.save_prefs()
}

// ---- Projects -------------------------------------------------------------------------------

#[tauri::command]
pub fn list_projects(state: AppStateRef) -> Result<Vec<ProjectSummary>, String> {
    let s = lock(&state)?;
    Ok(storage::list_projects(&s.projects_root()))
}

#[tauri::command]
pub fn projects_folder(state: AppStateRef) -> Result<PathBuf, String> {
    Ok(lock(&state)?.projects_root())
}

#[tauri::command]
pub fn create_project(name: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let root = s.projects_root();
    let dir = storage::create_project(&root, &name, s.prefs.new_project_defaults.defaults.clone())?;

    // Apply the rest of the app's new-project defaults (levels, backups) and save again.
    let mut project = storage::load_project(&dir)?;
    s.prefs.new_project_defaults.apply_to(&mut project);
    storage::save_project(&dir, &project)?;

    s.open_project(dir)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn open_project(path: PathBuf, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_project(path)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn close_project(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.close_project()?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Reloads comilito.json from disk, dropping unsaved changes (after a sync tool changed it).
#[tauri::command]
pub fn reload_project(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let dir = s.open_ref()?.dir.clone();
    s.open_mut()?.dirty_since = None;
    s.open_project(dir)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// After comilito.json changed on disk: keep what's in the app, overwriting the file.
#[tauri::command]
pub fn keep_my_version(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?.changed_on_disk = false;
    s.project_changed(Duration::ZERO);
    s.save_if_dirty()?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn rename_project(name: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("the project needs a name".into());
    }
    let mut s = lock(&state)?;
    s.open_mut()?.project.name = name;
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

// ---- Project settings -----------------------------------------------------------------------

#[tauri::command]
pub fn set_master_volume(volume: f32, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?.project.master_volume = volume.clamp(0.0, 1.0);
    let problems = s.levels_changed();
    emit_state(&app, &s, problems);
    Ok(())
}

/// `category` arrives from JS as "music", "ambience" or "effects"; Tauri rejects anything
/// else before this function runs, because `Category` can't be built from other strings.
#[tauri::command]
pub fn set_category_volume(
    category: Category,
    volume: f32,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?.project.categories.get_mut(category).volume = volume.clamp(0.0, 1.0);
    let problems = s.levels_changed();
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn set_project_defaults(
    defaults: ProjectDefaults,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?.project.defaults = defaults;
    let problems = s.levels_changed();
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn set_backup_settings(
    backups: BackupSettings,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?.project.backups = backups;
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// "Reset to my app defaults" in Project settings.
#[tauri::command]
pub fn reset_project_defaults(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let defaults = s.prefs.new_project_defaults.clone();
    defaults.apply_to(&mut s.open_mut()?.project);
    let problems = s.levels_changed();
    emit_state(&app, &s, problems);
    Ok(())
}

// ---- Sounds ---------------------------------------------------------------------------------

/// Copies audio files into the project and adds them to the library.
#[tauri::command]
pub fn add_sounds(
    category: Category,
    paths: Vec<PathBuf>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut problems = Vec::new();

    for path in paths {
        match storage::copy_audio_into_project(&open.dir, category, &path) {
            Ok(file) => {
                let name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Sound".into());
                open.project.sounds.push(Sound {
                    id: new_id("snd"),
                    category,
                    name,
                    file,
                    volume: None,
                    looping: None,
                    fade_in_ms: None,
                    fade_out_ms: None,
                });
            }
            Err(e) => problems.push(e),
        }
    }

    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(())
}

/// Updates a sound's name and settings. The file and category can't be changed this way.
#[tauri::command]
pub fn update_sound(sound: Sound, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let existing = s
        .open_mut()?
        .project
        .sound_mut(&sound.id)
        .ok_or("that sound no longer exists")?;
    existing.name = sound.name;
    existing.volume = sound.volume.map(|v| v.clamp(0.0, 1.0));
    existing.looping = sound.looping;
    existing.fade_in_ms = sound.fade_in_ms;
    existing.fade_out_ms = sound.fade_out_ms;
    let problems = s.levels_changed();
    emit_state(&app, &s, problems);
    Ok(())
}

/// Removes a sound from the library and every scene, and moves its file to the system
/// trash (or deletes it for good when `permanently` is true). See decision 0007.
#[tauri::command]
pub fn remove_sound(
    id: String,
    permanently: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let index = open
        .project
        .sounds
        .iter()
        .position(|snd| snd.id == id)
        .ok_or("that sound no longer exists")?;

    // `remove` takes the sound out of the Vec and gives us ownership of it.
    let sound = open.project.sounds.remove(index);
    open.project.remove_dangling_references();
    open.missing.retain(|m| m != &id);

    let path = storage::absolute_audio_path(&open.dir, &sound.file);
    let mut problems = Vec::new();
    // Another sound may share the file (not normally, but a hand-edited project could).
    let shared = open.project.sounds.iter().any(|o| o.file.path == sound.file.path);
    if path.exists() && !shared {
        let result = if permanently {
            std::fs::remove_file(&path).map_err(|e| e.to_string())
        } else {
            trash::delete(&path).map_err(|e| e.to_string())
        };
        if let Err(e) = result {
            problems.push(format!("could not remove {}: {e}", path.display()));
        }
    }

    problems.extend(s.project_changed(Duration::from_millis(500)));
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn preview_sound(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    // Split the borrow: `s.open` (read) and `s.engine` (write) are separate fields.
    let state_ref = &mut *s;
    let open = state_ref.open.as_ref().ok_or("no project is open")?;
    let sound = open.project.sound(&id).ok_or("that sound no longer exists")?;
    state_ref.engine.toggle_preview(&open.project, &open.dir, sound)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn play_effect(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let state_ref = &mut *s;
    let open = state_ref.open.as_ref().ok_or("no project is open")?;
    let sound = open.project.sound(&id).ok_or("that sound no longer exists")?;
    state_ref.engine.play_effect(&open.project, &open.dir, sound)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

// ---- Scenes ---------------------------------------------------------------------------------

/// Creates an empty scene and returns its id.
#[tauri::command]
pub fn create_scene(
    name: String,
    group: Option<String>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<String, String> {
    let mut s = lock(&state)?;
    let id = new_id("scn");
    s.open_mut()?.project.scenes.push(Scene {
        id: id.clone(),
        name,
        group: group.filter(|g| !g.trim().is_empty()),
        hotkey: None,
        music: None,
        ambience: Vec::new(),
        effects: Vec::new(),
        duck_level: None,
        transition_ms: None,
    });
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(id)
}

/// Replaces a scene with the edited version from the scene editor.
#[tauri::command]
pub fn update_scene(scene: Scene, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let existing = open
        .project
        .scenes
        .iter_mut()
        .find(|sc| sc.id == scene.id)
        .ok_or("that scene no longer exists")?;
    let transition = millis(scene.transition_ms.unwrap_or(open.project.defaults.transition_ms));
    *existing = scene;
    // Drop references to sounds that don't exist, in case the UI sent a stale id.
    open.project.remove_dangling_references();
    let problems = s.project_changed(transition.min(Duration::from_millis(500)));
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn duplicate_scene(id: String, app: AppHandle, state: AppStateRef) -> Result<String, String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut copy = open
        .project
        .scene(&id)
        .ok_or("that scene no longer exists")?
        .clone();
    copy.id = new_id("scn");
    copy.name = format!("{} (copy)", copy.name);
    copy.hotkey = None;
    let new_scene_id = copy.id.clone();
    open.project.scenes.push(copy);
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(new_scene_id)
}

/// Deletes a scene, and removes it from the stack and every preset.
#[tauri::command]
pub fn delete_scene(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let transition = open
        .project
        .scene(&id)
        .map(|sc| millis(open.project.scene_transition_ms(sc)))
        .unwrap_or_default();
    open.project.scenes.retain(|sc| sc.id != id);
    open.project.remove_dangling_references();
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

/// A live slider in an expanded stack entry: changes that layer's volume in its scene.
#[tauri::command]
pub fn set_layer_volume(
    scene_id: String,
    layer_index: usize,
    volume: f32,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let scene = open
        .project
        .scenes
        .iter_mut()
        .find(|sc| sc.id == scene_id)
        .ok_or("that scene no longer exists")?;
    let layer = scene
        .ambience
        .get_mut(layer_index)
        .ok_or("that layer no longer exists")?;
    layer.volume = Some(volume.clamp(0.0, 1.0));
    let problems = s.levels_changed();
    emit_state(&app, &s, problems);
    Ok(())
}

// ---- The stack ------------------------------------------------------------------------------
// Stack indexes count from the bottom (0 = bottom), like the `stack` list in comilito.json.
// The UI shows the top first and converts.

/// The fade time for adding/removing/toggling a scene: that scene's `transitionMs`.
fn scene_transition(s: &AppState, scene_id: &str) -> Duration {
    s.open
        .as_ref()
        .and_then(|o| o.project.scene(scene_id).map(|sc| o.project.scene_transition_ms(sc)))
        .map(millis)
        .unwrap_or_default()
}

/// For switches and presets: the top scene's `transitionMs`.
fn top_transition(s: &AppState) -> Duration {
    let top = s.open.as_ref().and_then(|o| o.project.stack.last());
    match top {
        Some(entry) => scene_transition(s, &entry.scene),
        None => Duration::from_millis(1000),
    }
}

/// Adds a scene to the stack at `index`. With `mode = "move"`, a scene already in the stack
/// is moved there instead of duplicated (the UI asks which; see docs/scenes.md).
#[tauri::command]
pub fn stack_insert(
    scene_id: String,
    index: usize,
    mode: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let transition = scene_transition(&s, &scene_id);
    let stack = &mut s.open_mut()?.project.stack;

    let mut index = index.min(stack.len());
    let existing = stack.iter().position(|e| e.scene == scene_id);
    let entry = match (mode.as_str(), existing) {
        ("move", Some(old)) => {
            // Removing an entry below the target shifts the target down by one.
            if old < index {
                index -= 1;
            }
            let mut entry = stack.remove(old);
            entry.enabled = true;
            entry
        }
        _ => StackEntry {
            id: new_id("stk"),
            scene: scene_id,
            enabled: true,
        },
    };
    stack.insert(index, entry);

    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn stack_move(
    entry_id: String,
    index: usize,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let stack = &s.open_ref()?.project.stack;
    let old = stack
        .iter()
        .position(|e| e.id == entry_id)
        .ok_or("that stack entry no longer exists")?;
    let scene_id = stack[old].scene.clone();
    let transition = scene_transition(&s, &scene_id);

    let stack = &mut s.open_mut()?.project.stack;
    let entry = stack.remove(old);
    let index = if old < index { index - 1 } else { index };
    stack.insert(index.min(stack.len()), entry);

    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn stack_remove(entry_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let scene_id = s
        .open_ref()?
        .project
        .stack
        .iter()
        .find(|e| e.id == entry_id)
        .map(|e| e.scene.clone())
        .ok_or("that stack entry no longer exists")?;
    let transition = scene_transition(&s, &scene_id);
    s.open_mut()?.project.stack.retain(|e| e.id != entry_id);
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn stack_set_enabled(
    entry_id: String,
    enabled: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let entry = s
        .open_mut()?
        .project
        .stack
        .iter_mut()
        .find(|e| e.id == entry_id)
        .ok_or("that stack entry no longer exists")?;
    entry.enabled = enabled;
    let scene_id = entry.scene.clone();
    let transition = scene_transition(&s, &scene_id);
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

/// "Switch stack to this scene": the stack becomes just this scene.
#[tauri::command]
pub fn stack_switch(scene_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?.project.stack = vec![StackEntry {
        id: new_id("stk"),
        scene: scene_id,
        enabled: true,
    }];
    let transition = top_transition(&s);
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

/// A scene's hotkey: adds it on top if it isn't in the stack, otherwise toggles it
/// (the top-most copy, if it's there twice).
#[tauri::command]
pub fn scene_hotkey(scene_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let transition = scene_transition(&s, &scene_id);
    let stack = &mut s.open_mut()?.project.stack;
    match stack.iter_mut().rev().find(|e| e.scene == scene_id) {
        Some(entry) => entry.enabled = !entry.enabled,
        None => stack.push(StackEntry {
            id: new_id("stk"),
            scene: scene_id,
            enabled: true,
        }),
    }
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

// ---- Presets --------------------------------------------------------------------------------

#[tauri::command]
pub fn preset_save(name: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    let stack = project
        .stack
        .iter()
        .map(|e| PresetEntry {
            scene: e.scene.clone(),
            enabled: e.enabled,
        })
        .collect();
    project.stack_presets.push(StackPreset {
        id: new_id("pre"),
        name,
        stack,
    });
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Turns a preset's entries into new stack entries (each gets a fresh id).
fn preset_entries(s: &AppState, preset_id: &str) -> Result<Vec<StackEntry>, String> {
    let preset = s
        .open_ref()?
        .project
        .stack_presets
        .iter()
        .find(|p| p.id == preset_id)
        .ok_or("that preset no longer exists")?;
    Ok(preset
        .stack
        .iter()
        .map(|e| StackEntry {
            id: new_id("stk"),
            scene: e.scene.clone(),
            enabled: e.enabled,
        })
        .collect())
}

/// "Replace stack" with a preset.
#[tauri::command]
pub fn preset_load(preset_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let entries = preset_entries(&s, &preset_id)?;
    s.open_mut()?.project.stack = entries;
    let transition = top_transition(&s);
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Dragging a preset into the stack inserts its scenes at the drop position.
#[tauri::command]
pub fn preset_insert(
    preset_id: String,
    index: usize,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let entries = preset_entries(&s, &preset_id)?;
    let stack = &mut s.open_mut()?.project.stack;
    let index = index.min(stack.len());
    // `splice` replaces a range (here an empty one at `index`) with new items: an insert of many.
    stack.splice(index..index, entries);
    let transition = top_transition(&s);
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn preset_rename(
    preset_id: String,
    name: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let preset = s
        .open_mut()?
        .project
        .stack_presets
        .iter_mut()
        .find(|p| p.id == preset_id)
        .ok_or("that preset no longer exists")?;
    preset.name = name;
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn preset_delete(preset_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.open_mut()?
        .project
        .stack_presets
        .retain(|p| p.id != preset_id);
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

// ---- Transport ------------------------------------------------------------------------------

/// Space: start, pause or resume.
#[tauri::command]
pub fn toggle_play(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let pause_fade = millis(s.prefs.playback.pause_fade_ms);
    let state_ref = &mut *s;
    let open = state_ref.open.as_ref().ok_or("no project is open")?;
    let problems = state_ref
        .engine
        .toggle_play(&open.project, &open.dir, &open.mix, pause_fade);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Esc: fade everything out. The stack is kept; Space starts it again from the beginning.
#[tauri::command]
pub fn stop_all(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let fade = millis(s.prefs.playback.stop_fade_ms);
    s.engine.stop_all(fade);
    emit_state(&app, &s, Vec::new());
    Ok(())
}
