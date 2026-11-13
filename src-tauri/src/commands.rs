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

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    bundle,
    prefs::Preferences,
    project::{
        BackupSettings, Category, Layer, Playlist, PresetEntry, ProjectDefaults, Scene, Sound, StackEntry,
        StackPreset, folder_of_path, new_id,
    },
    state::{AppState, OpenProject, Snapshot, emit_state},
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

/// The current time as stored in comilito.json (RFC 3339, UTC).
fn now() -> String {
    storage::format_time(std::time::SystemTime::now())
}

/// Remembers that a sound was just used, for the Library's "Recent" list.
fn touch(open: &mut OpenProject, sound_id: &str) {
    if let Some(sound) = open.project.sound_mut(sound_id) {
        sound.last_used = Some(now());
        open.dirty_since.get_or_insert_with(std::time::Instant::now);
    }
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

// ---- Appearance -----------------------------------------------------------------------------

#[tauri::command]
pub fn desktop_appearance() -> crate::desktop::DesktopAppearance {
    crate::desktop::read()
}

#[tauri::command]
pub fn list_user_themes(state: AppStateRef) -> Result<Vec<crate::themes::Theme>, String> {
    Ok(crate::themes::list(&lock(&state)?.config_dir))
}

#[tauri::command]
pub fn save_user_theme(theme: crate::themes::Theme, state: AppStateRef) -> Result<(), String> {
    crate::themes::save(&lock(&state)?.config_dir, &theme)
}

#[tauri::command]
pub fn delete_user_theme(id: String, state: AppStateRef) -> Result<(), String> {
    crate::themes::delete(&lock(&state)?.config_dir, &id)
}

#[tauri::command]
pub fn import_theme(path: PathBuf, state: AppStateRef) -> Result<crate::themes::Theme, String> {
    crate::themes::import(&lock(&state)?.config_dir, &path)
}

#[tauri::command]
pub fn export_theme(theme: crate::themes::Theme, dest: PathBuf) -> Result<(), String> {
    crate::themes::export(&theme, &dest)
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

/// Creates a project folder and opens it. With `sample`, it starts with the bundled sample
/// sounds and scenes (see sample.rs) instead of empty.
#[tauri::command]
pub fn create_project(
    name: String,
    sample: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    // Check before creating anything, so a failure doesn't leave an empty folder behind.
    let sample_dir = match (sample, &s.sample_dir) {
        (true, None) => return Err("the sample audio wasn't found in this installation".into()),
        (true, Some(d)) => Some(d.clone()),
        (false, _) => None,
    };
    let root = s.projects_root();
    let dir = storage::create_project(&root, &name, s.prefs.new_project_defaults.defaults.clone())?;

    // Apply the rest of the app's new-project defaults (levels, backups) and save again.
    let mut project = storage::load_project(&dir)?;
    s.prefs.new_project_defaults.apply_to(&mut project);
    if let Some(sample_dir) = sample_dir {
        crate::sample::fill_sample_project(&dir, &sample_dir, &mut project)?;
    }
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

/// Deletes a project. With `delete_audio`, the whole folder goes to the system trash; without,
/// only its comilito.json does, leaving the audio files in place (the folder then no longer
/// counts as a project). Backups are kept either way.
#[tauri::command]
pub fn delete_project(
    path: PathBuf,
    delete_audio: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    // Refuse anything that isn't a project folder, so a wrong path can't trash something else.
    let project_file = path.join(storage::PROJECT_FILE);
    if !project_file.is_file() {
        return Err(format!("{} is not a Comilito project", path.display()));
    }
    if s.open.as_ref().is_some_and(|o| o.dir == path) {
        s.close_project()?;
    }
    let target = if delete_audio { &path } else { &project_file };
    trash::delete(target).map_err(|e| format!("could not move {} to the trash: {e}", target.display()))?;
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

/// Exports the open project as a `.comilito` zip, with or without its audio.
#[tauri::command]
pub fn export_project(
    dest: PathBuf,
    include_audio: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let s = lock(&state)?;
    let open = s.open_ref()?;
    bundle::export_zip(&open.dir, &open.project, &dest, include_audio)?;
    let _ = app.emit("notice", format!("Exported to {}", dest.display()));
    Ok(())
}

/// Unpacks a `.comilito` zip (an export or a backup) into a new project folder and opens it.
#[tauri::command]
pub fn import_project(path: PathBuf, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let dir = bundle::import_zip(&path, &s.projects_root())?;
    s.open_project(dir)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn backups_folder(state: AppStateRef) -> Result<PathBuf, String> {
    Ok(lock(&state)?.backups_root())
}

/// Looks for every missing sound's file (same name) under `folder`, and copies the matches
/// into the project. One folder can fix many sounds at once.
#[tauri::command]
pub fn relink_from_folder(folder: PathBuf, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut fixed = 0;
    let mut problems = Vec::new();

    for id in open.missing.clone() {
        let Some(sound) = open.project.sound(&id) else { continue };
        let file_name = sound.file.path.rsplit('/').next().unwrap_or_default().to_string();
        let candidates = bundle::find_by_name(&folder, &file_name);
        let Some(found) = bundle::best_candidate(candidates, sound.file.size, sound.file.sha256.as_deref()) else {
            continue;
        };
        match relink(open, &id, &found) {
            Ok(()) => fixed += 1,
            Err(e) => problems.push(e),
        }
    }

    let still = s.open_ref()?.missing.len();
    problems.push(format!("Relinked {fixed} sound(s); {still} still missing."));
    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(())
}

/// Points one missing sound at a file the user picked.
#[tauri::command]
pub fn relink_sound(id: String, path: PathBuf, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    relink(s.open_mut()?, &id, &path)?;
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Copies `source` into the project for sound `id` and updates its file info.
fn relink(open: &mut OpenProject, id: &str, source: &std::path::Path) -> Result<(), String> {
    let sound = open.project.sound(id).ok_or("that sound no longer exists")?;
    let category = bundle::category_of_path(&sound.file.path, sound.category);
    let folder = folder_of_path(&sound.file.path);
    let file = storage::copy_audio_into_project(&open.dir, category, &folder, source)?;
    if let Some(sound) = open.project.sound_mut(id) {
        sound.file = file;
    }
    open.missing.retain(|m| m != id);
    // A new file gets measured again.
    open.analysis_failed.retain(|m| m != id);
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

/// Copies audio files into the project (into a Library folder, if given) and adds them to
/// the library.
#[tauri::command]
pub fn add_sounds(
    category: Category,
    folder: Option<String>,
    paths: Vec<PathBuf>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut problems = Vec::new();
    let folder = folder.unwrap_or_default();

    for path in paths {
        match storage::copy_audio_into_project(&open.dir, category, &folder, &path) {
            Ok(file) => {
                let name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Sound".into());
                open.project.sounds.push(Sound::new(category, name, file));
            }
            Err(e) => problems.push(e),
        }
    }

    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(())
}

/// Updates sounds' names, settings, tags and favourite flags (one sound from the editor, or
/// several from a bulk edit in the Library). Files, categories and folders can't be changed
/// this way; see `move_sounds`.
#[tauri::command]
pub fn update_sounds(sounds: Vec<Sound>, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    for sound in sounds {
        let Some(existing) = open.project.sound_mut(&sound.id) else {
            continue;
        };
        let name = sound.name.trim();
        if !name.is_empty() {
            existing.name = name.to_string();
        }
        existing.volume = sound.volume.map(|v| v.clamp(0.0, 1.0));
        existing.looping = sound.looping;
        existing.fade_in_ms = sound.fade_in_ms;
        existing.fade_out_ms = sound.fade_out_ms;
        existing.favourite = sound.favourite;
        // Tags are stored without `#`, trimmed, lower-case and without duplicates.
        let mut tags: Vec<String> = sound
            .tags
            .iter()
            .map(|t| t.trim().trim_start_matches('#').to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        tags.dedup();
        existing.tags = tags;
    }
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
    touch(s.open_mut()?, &id);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn toggle_effect(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let state_ref = &mut *s;
    let open = state_ref.open.as_ref().ok_or("no project is open")?;
    let sound = open.project.sound(&id).ok_or("that sound no longer exists")?;
    state_ref.engine.toggle_effect(&open.project, &open.dir, sound)?;
    touch(s.open_mut()?, &id);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Moves sounds to a category and Library folder: their files move to
/// `audio/<category>/<folder>/`, and they're taken out of scene slots that don't take that
/// category (a music sound turned into an effect leaves the playlists it was in).
#[tauri::command]
pub fn move_sounds(
    ids: Vec<String>,
    category: Category,
    folder: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut problems = Vec::new();

    for id in ids {
        let Some(sound) = open.project.sound(&id) else { continue };
        if sound.category == category && sound.folder() == folder {
            continue;
        }
        let new_file = if open.missing.contains(&id) {
            // No file to move; it will be found again by relinking.
            None
        } else {
            match storage::move_audio_file(&open.dir, &sound.file, category, &folder) {
                Ok(file) => Some(file),
                Err(e) => {
                    problems.push(e);
                    continue;
                }
            }
        };
        if let Some(sound) = open.project.sound_mut(&id) {
            sound.category = category;
            if let Some(file) = new_file {
                sound.file = file;
            }
        }
        open.project.remove_sound_from_other_slots(&id, category);
    }
    open.folders = storage::list_folders(&open.dir);
    s.engine.clear_cache();

    problems.extend(s.project_changed(Duration::from_millis(500)));
    emit_state(&app, &s, problems);
    Ok(())
}

// ---- Library folders ------------------------------------------------------------------------

#[tauri::command]
pub fn create_folder(category: Category, folder: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    storage::create_folder(&open.dir, category, folder.trim())?;
    open.folders = storage::list_folders(&open.dir);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Renames a Library folder and points every sound inside it at its new path.
#[tauri::command]
pub fn rename_folder(
    category: Category,
    from: String,
    to: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let to = to.trim().trim_matches('/').to_string();
    storage::rename_folder(&open.dir, category, &from, &to)?;
    // The folder on disk now has its final name; rebuild each moved sound's stored path.
    let old_prefix = format!("audio/{}/{}/", category.folder_name(), from);
    let new_root = storage::category_folder_path(&open.dir, category, &to);
    let new_prefix = format!(
        "{}/",
        storage::to_json_path(new_root.strip_prefix(&open.dir).unwrap_or(&new_root))
    );
    for sound in &mut open.project.sounds {
        if let Some(rest) = sound.file.path.strip_prefix(&old_prefix) {
            sound.file.path = format!("{new_prefix}{rest}");
        }
    }
    open.folders = storage::list_folders(&open.dir);
    s.engine.clear_cache();
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn delete_folder(category: Category, folder: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    storage::delete_empty_folder(&open.dir, category, &folder)?;
    open.folders = storage::list_folders(&open.dir);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Opens a Library folder (or the whole `audio/` folder) in the system file manager.
#[tauri::command]
pub fn open_audio_folder(
    category: Option<Category>,
    folder: Option<String>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let s = lock(&state)?;
    let dir = &s.open_ref()?.dir;
    let path = match category {
        Some(c) => storage::category_folder_path(dir, c, folder.as_deref().unwrap_or("")),
        None => dir.join("audio"),
    };
    std::fs::create_dir_all(&path).map_err(|e| format!("could not create {}: {e}", path.display()))?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("could not open the file manager: {e}"))
}

/// Shows a sound's file selected in the system file manager.
#[tauri::command]
pub fn reveal_sound(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let s = lock(&state)?;
    let open = s.open_ref()?;
    let sound = open.project.sound(&id).ok_or("that sound no longer exists")?;
    let path = storage::absolute_audio_path(&open.dir, &sound.file);
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| format!("could not open the file manager: {e}"))
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
        blocks_music: false,
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

/// Dropping a sound on a scene in the stack: adds it where its category belongs (music to the
/// playlist, ambience as a layer, effects to the pad). Does nothing if it's already there.
#[tauri::command]
pub fn add_sound_to_scene(
    scene_id: String,
    sound_id: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let category = open
        .project
        .sound(&sound_id)
        .ok_or("that sound no longer exists")?
        .category;
    touch(open, &sound_id);
    let scene = open
        .project
        .scenes
        .iter_mut()
        .find(|sc| sc.id == scene_id)
        .ok_or("that scene no longer exists")?;

    match category {
        Category::Music => {
            // `get_or_insert_with` creates the playlist if the scene had none.
            let playlist = scene.music.get_or_insert_with(|| Playlist {
                tracks: Vec::new(),
                shuffle: None,
            });
            if !playlist.tracks.contains(&sound_id) {
                playlist.tracks.push(sound_id);
            }
        }
        Category::Ambience => {
            if !scene.ambience.iter().any(|l| l.sound == sound_id) {
                scene.ambience.push(Layer {
                    sound: sound_id,
                    volume: None,
                    role: None,
                });
            }
        }
        Category::Effects => {
            if !scene.effects.contains(&sound_id) {
                scene.effects.push(sound_id);
            }
        }
    }
    let transition = millis(open.project.defaults.transition_ms.min(1000));
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

/// The ✕ next to a sound in an expanded stack entry: takes it out of that scene.
/// The sound stays in the library.
#[tauri::command]
pub fn remove_sound_from_scene(
    scene_id: String,
    sound_id: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let transition = open
        .project
        .scene(&scene_id)
        .map(|sc| millis(open.project.scene_transition_ms(sc)))
        .ok_or("that scene no longer exists")?;
    open.project.remove_sound_from_scene(&scene_id, &sound_id);
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Removes every sound whose audio file is missing (from the library and all scenes).
/// For when files were deleted on purpose outside the app.
#[tauri::command]
pub fn remove_missing_sounds(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let missing = std::mem::take(&mut open.missing);
    open.project.sounds.retain(|snd| !missing.contains(&snd.id));
    open.project.remove_dangling_references();
    let problems = s.project_changed(Duration::from_millis(500));
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
    let project = &mut s.open_mut()?.project;
    project.stack = vec![StackEntry {
        id: new_id("stk"),
        scene: scene_id,
        enabled: true,
    }];
    project.stack_origin = None;
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

/// "Save as new preset": stores the current stack and makes it the stack's origin.
#[tauri::command]
pub fn preset_save(name: String, group: Option<String>, app: AppHandle, state: AppStateRef) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("the preset needs a name".into());
    }
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    let id = new_id("pre");
    let group = group.map(|g| g.trim().to_string()).filter(|g| !g.is_empty());
    if let Some(g) = &group
        && !project.preset_groups.contains(g)
    {
        project.preset_groups.push(g.clone());
    }
    let stack = project.stack_as_preset_entries();
    project.stack_presets.push(StackPreset {
        id: id.clone(),
        name,
        group,
        stack,
    });
    project.stack_origin = Some(id.clone());
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(id)
}

/// "Update <preset>": overwrites a preset with the current stack.
#[tauri::command]
pub fn preset_update(preset_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    let stack = project.stack_as_preset_entries();
    let preset = project
        .stack_presets
        .iter_mut()
        .find(|p| p.id == preset_id)
        .ok_or("that preset no longer exists")?;
    preset.stack = stack;
    project.stack_origin = Some(preset_id);
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Replaces a preset's entries (editing a preset in place, decision 0017).
#[tauri::command]
pub fn preset_set_entries(
    preset_id: String,
    entries: Vec<PresetEntry>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    let preset = project
        .stack_presets
        .iter_mut()
        .find(|p| p.id == preset_id)
        .ok_or("that preset no longer exists")?;
    preset.stack = entries;
    project.remove_dangling_references();
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Turns a preset's entries into new stack entries (each gets a fresh id).
fn preset_entries(s: &AppState, preset_id: &str) -> Result<Vec<StackEntry>, String> {
    let preset = s
        .open_ref()?
        .project
        .preset(preset_id)
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

/// "Replace stack" with a preset (also used by "Revert to <preset>").
#[tauri::command]
pub fn preset_load(preset_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let entries = preset_entries(&s, &preset_id)?;
    let project = &mut s.open_mut()?.project;
    project.stack = entries;
    project.stack_origin = Some(preset_id);
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

/// Finds a preset to change it.
fn preset_mut<'a>(s: &'a mut AppState, preset_id: &str) -> Result<&'a mut StackPreset, String> {
    s.open_mut()?
        .project
        .stack_presets
        .iter_mut()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| "that preset no longer exists".to_string())
}

#[tauri::command]
pub fn preset_rename(
    preset_id: String,
    name: String,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("the preset needs a name".into());
    }
    let mut s = lock(&state)?;
    preset_mut(&mut s, &preset_id)?.name = name;
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn preset_duplicate(preset_id: String, app: AppHandle, state: AppStateRef) -> Result<String, String> {
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    let mut copy = project.preset(&preset_id).ok_or("that preset no longer exists")?.clone();
    copy.id = new_id("pre");
    copy.name = format!("{} (copy)", copy.name);
    let id = copy.id.clone();
    // Right after the original.
    let index = project.stack_presets.iter().position(|p| p.id == preset_id).map_or(0, |i| i + 1);
    project.stack_presets.insert(index, copy);
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(id)
}

/// "Move to group": `None` (or an empty name) means Ungrouped.
#[tauri::command]
pub fn preset_set_group(
    preset_id: String,
    group: Option<String>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let group = group.map(|g| g.trim().to_string()).filter(|g| !g.is_empty());
    let project = &mut s.open_mut()?.project;
    if let Some(g) = &group
        && !project.preset_groups.contains(g)
    {
        project.preset_groups.push(g.clone());
    }
    preset_mut(&mut s, &preset_id)?.group = group;
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn preset_delete(preset_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    project.stack_presets.retain(|p| p.id != preset_id);
    project.remove_dangling_references();
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn preset_group_create(name: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("the group needs a name".into());
    }
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    if project.all_preset_groups().contains(&name) {
        return Err(format!("there's already a group named \"{name}\""));
    }
    project.preset_groups.push(name);
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Renames a preset group (and every preset in it).
#[tauri::command]
pub fn preset_group_rename(from: String, to: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let to = to.trim().to_string();
    if to.is_empty() {
        return Err("the group needs a name".into());
    }
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    if to != from && project.all_preset_groups().contains(&to) {
        return Err(format!("there's already a group named \"{to}\""));
    }
    project.preset_groups = project.all_preset_groups();
    for g in &mut project.preset_groups {
        if *g == from {
            *g = to.clone();
        }
    }
    for preset in &mut project.stack_presets {
        if preset.group.as_deref() == Some(from.as_str()) {
            preset.group = Some(to.clone());
        }
    }
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Deletes a preset group. Its presets are kept, ungrouped.
#[tauri::command]
pub fn preset_group_delete(name: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let project = &mut s.open_mut()?.project;
    project.preset_groups.retain(|g| *g != name);
    for preset in &mut project.stack_presets {
        if preset.group.as_deref() == Some(name.as_str()) {
            preset.group = None;
        }
    }
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// "Clear tapestry": empties the stack (everything fades out).
#[tauri::command]
pub fn stack_clear(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let transition = top_transition(&s);
    let project = &mut s.open_mut()?.project;
    project.stack.clear();
    project.stack_origin = None;
    let problems = s.project_changed(transition);
    emit_state(&app, &s, problems);
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
