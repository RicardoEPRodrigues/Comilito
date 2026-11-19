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
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    bundle,
    prefs::Preferences,
    project::{
        BackupSettings, Category, Layer, Playlist, PresetEntry, ProjectDefaults, Role, Scene, Shelf, Sound, StackEntry,
        StackPreset, folder_of_path, new_id,
    },
    state::{AppState, OpenProject, Snapshot, emit_state},
    storage::{self, ProjectSummary},
};

/// The type every command receives to reach the shared state.
/// `'_` is an anonymous *lifetime*: it says "this borrows something Tauri owns, for as long
/// as the command runs". You mostly see lifetimes when a type holds a reference.
pub type AppStateRef<'a> = State<'a, Mutex<AppState>>;

/// Locks the state. `lock()` fails only if a thread panicked while holding the lock.
/// The guard unlocks automatically when it goes out of scope (like C++ RAII).
pub fn lock<'a>(state: &'a AppStateRef) -> Result<MutexGuard<'a, AppState>, String> {
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

/// Quits after the user said yes to the "still busy" warning (see `on_window_event` in
/// lib.rs): marks the app as quitting, so the close isn't held back again, and closes it.
#[tauri::command]
pub fn confirm_quit(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    lock(&state)?.quitting = true;
    // The lock is released at the end of the line above: closing the window runs the
    // window-event handler, which takes it again. The video window must go too, or the app
    // wouldn't exit.
    crate::video_window::close(&app);
    match app.get_webview_window("main") {
        Some(window) => window.close().map_err(|e| e.to_string()),
        None => {
            app.exit(0);
            Ok(())
        }
    }
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
pub fn set_preferences(mut prefs: Preferences, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    // Rust keeps the project lists (see Preferences::recent_projects).
    prefs.recent_projects = std::mem::take(&mut s.prefs.recent_projects);
    prefs.other_projects = std::mem::take(&mut s.prefs.other_projects);
    let output_changed = prefs.playback.output_device != s.prefs.playback.output_device;
    let video_turned_off = s.prefs.video.enabled && !prefs.video.enabled;
    // The screen or fullscreen changed in Preferences: an open video window follows now.
    let video_moved = s.prefs.video.screen != prefs.video.screen || s.prefs.video.fullscreen != prefs.video.fullscreen;
    s.prefs = prefs;
    s.save_prefs()?;
    // The video window (decision 0035) follows its settings live.
    let _ = app.emit("preferences-changed", &s.prefs);
    if video_turned_off {
        crate::video_window::close(&app);
    } else if video_moved {
        let _ = crate::video_window::place(&app, s.prefs.video.screen.as_deref(), s.prefs.video.fullscreen);
    }
    if output_changed {
        let device = s.prefs.playback.output_device.clone();
        let problems = s.switch_output(device.as_deref());
        emit_state(&app, &s, problems);
    }
    Ok(())
}

/// The audio outputs to choose from in Preferences › Playback.
#[tauri::command]
pub fn list_output_devices() -> crate::audio::OutputDevices {
    crate::audio::output_devices()
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
pub fn export_theme(theme: crate::themes::Theme, dest: PathBuf, state: AppStateRef) -> Result<(), String> {
    // Clone the path so the state lock is released before the (slower) file work.
    let config_dir = lock(&state)?.config_dir.clone();
    crate::themes::export(&config_dir, &theme, &dest)
}

/// Stores a file picked in the theme editor (`role`: "watermark", "font" or "licence") and
/// returns the name the theme should refer to it by.
#[tauri::command]
pub fn add_theme_file(path: PathBuf, role: crate::themes::Role, state: AppStateRef) -> Result<String, String> {
    let config_dir = lock(&state)?.config_dir.clone();
    crate::themes::add_file(&config_dir, &path, role)
}

/// A file a user theme brought with it (a font or its watermark), as raw bytes.
/// `ipc::Response` sends them as binary, which arrives in JavaScript as an `ArrayBuffer`
/// instead of a (much bigger, slower) JSON array of numbers.
#[tauri::command]
pub fn read_theme_asset(name: String, state: AppStateRef) -> Result<tauri::ipc::Response, String> {
    let config_dir = lock(&state)?.config_dir.clone();
    crate::themes::read_asset(&config_dir, &name).map(tauri::ipc::Response::new)
}

// ---- Projects -------------------------------------------------------------------------------

#[tauri::command]
pub fn list_projects(state: AppStateRef) -> Result<Vec<ProjectSummary>, String> {
    let s = lock(&state)?;
    Ok(storage::list_projects(&s.projects_root(), &s.prefs.other_projects))
}

/// "Remove from list" on the home screen: forgets a project opened from elsewhere, leaving
/// its folder alone (decision 0042).
#[tauri::command]
pub fn forget_project(path: PathBuf, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.prefs.forget_project(&path);
    s.save_prefs()
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
    // "Open a project folder…" can pick any folder: say plainly when it isn't a project.
    if !path.join(storage::PROJECT_FILE).is_file() {
        return Err(format!(
            "“{}” isn't a Comilito project: it has no {}. To bring audio in, create a project and add the files.",
            path.display(),
            storage::PROJECT_FILE
        ));
    }
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

/// Opens the projects folder in the file manager, or with `project`, shows that project's
/// folder selected in it. Only project folders are accepted, so this can't open anything else.
#[tauri::command]
pub fn show_projects_folder(project: Option<PathBuf>, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let root = lock(&state)?.projects_root();
    let result = match project {
        Some(dir) if dir.join(storage::PROJECT_FILE).is_file() => app.opener().reveal_item_in_dir(&dir),
        Some(dir) => return Err(format!("{} is not a Comilito project", dir.display())),
        None => {
            std::fs::create_dir_all(&root).map_err(|e| format!("could not create {}: {e}", root.display()))?;
            app.opener().open_path(root.to_string_lossy(), None::<&str>)
        }
    };
    result.map_err(|e| format!("could not open the file manager: {e}"))
}

/// Makes a copy of a project folder, named "<name> (copy)", next to it. Returns the new folder.
#[tauri::command]
pub fn duplicate_project(path: PathBuf, state: AppStateRef) -> Result<PathBuf, String> {
    let mut s = lock(&state)?;
    s.not_moving()?;
    // Copy what's on screen, not what was last saved.
    if s.open.as_ref().is_some_and(|o| o.dir == path) {
        s.save_if_dirty()?;
    }
    let copy = crate::relocate::duplicate_project(&path)?;
    crate::logging::info(&format!("Duplicated {} as {}", path.display(), copy.display()));
    // The copy is next to the original: a copy of a project from elsewhere is elsewhere too.
    if !storage::is_in_root(&copy, &s.projects_root()) && s.prefs.note_elsewhere(&copy) {
        s.save_prefs()?;
    }
    Ok(copy)
}

/// Progress of a projects-folder move, sent as `projects-move-progress`.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveProgress {
    done_bytes: u64,
    total_bytes: u64,
    /// The file or folder just moved.
    current: String,
}

/// The end of a move, sent as `projects-moved`: how many folders moved, or why it stopped.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveFinished {
    moved: Option<usize>,
    error: Option<String>,
}

/// Moves every project and the backups into another folder and makes it the projects folder
/// (Preferences › Projects, decision 0023).
///
/// Checking happens here, so mistakes (a name clash, moving into a project) come back as this
/// command's error. The move itself can take minutes across disks, so it runs on its own
/// thread and this returns at once; progress and the end arrive as events. The state lock is
/// only held briefly at the start and the end, so the app stays responsive meanwhile.
#[tauri::command]
pub fn move_projects_folder(to: PathBuf, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.not_moving()?;
    let from = s.projects_root();
    let plan = crate::relocate::plan_move(&from, &to)?;
    // The open project's files are about to move, so close it (saving and backing it up);
    // it's reopened from its new place at the end.
    let reopen = match &s.open {
        Some(open) if open.dir.starts_with(&from) => Some(open.dir.strip_prefix(&from).expect("checked").to_path_buf()),
        _ => None,
    };
    if reopen.is_some() {
        s.close_project()?;
    }
    s.moving_projects = true;
    emit_state(&app, &s, Vec::new());
    drop(s); // Unlock before the long part.

    std::thread::spawn(move || {
        let mut last_emit = std::time::Instant::now();
        let result = crate::relocate::run_move(&plan, &mut |done, current| {
            // A few updates a second is plenty for a progress bar.
            if last_emit.elapsed() >= Duration::from_millis(100) || done == plan.total_bytes {
                last_emit = std::time::Instant::now();
                let _ = app.emit(
                    "projects-move-progress",
                    MoveProgress { done_bytes: done, total_bytes: plan.total_bytes, current: current.display().to_string() },
                );
            }
        });

        // `app.state()` fetches the same Mutex<AppState> the commands get.
        let state = app.state::<Mutex<AppState>>();
        let Ok(mut s) = state.lock() else { return };
        s.moving_projects = false;
        let finished = match result {
            Ok(moved) => {
                crate::logging::info(&format!("Moved {moved} project folders from {} to {}", plan.from.display(), plan.to.display()));
                s.prefs.projects_folder = (plan.to != s.default_projects_root).then(|| plan.to.clone());
                s.prefs.moved_projects(&plan.from, &plan.to);
                match s.save_prefs() {
                    Ok(()) => MoveFinished { moved: Some(moved), error: None },
                    Err(e) => MoveFinished { moved: Some(moved), error: Some(e) },
                }
            }
            Err(e) => {
                crate::logging::warn(&format!("Moving the projects failed: {e}"));
                MoveFinished { moved: None, error: Some(e) }
            }
        };
        // Reopen it wherever it is now (back where it was if the move failed).
        if let Some(relative) = reopen {
            let dir = s.projects_root().join(relative);
            if let Err(e) = s.open_project(dir) {
                crate::logging::warn(&format!("could not reopen the project after moving: {e}"));
            }
        }
        emit_state(&app, &s, Vec::new());
        let _ = app.emit("projects-moved", finished);
    });
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
    s.not_moving()?;
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
    // Either way it's no longer a project: drop it from the lists.
    s.prefs.forget_project(&path);
    if let Err(e) = s.save_prefs() {
        crate::logging::warn(&format!("could not save the project lists: {e}"));
    }
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

/// Exports the open project as a `.comilito` zip, with or without its audio (and videos).
#[tauri::command]
pub fn export_project(
    dest: PathBuf,
    include_audio: bool,
    include_video: Option<bool>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let s = lock(&state)?;
    let open = s.open_ref()?;
    bundle::export_zip(&open.dir, &open.project, &dest, include_audio, include_video.unwrap_or(false))?;
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
        // Sounds and videos share the missing list; each is relinked its own way.
        let (file, is_video) = match (open.project.sound(&id), open.project.video(&id)) {
            (Some(sound), _) => (sound.file.clone(), false),
            (None, Some(video)) => (video.file.clone(), true),
            (None, None) => continue,
        };
        let file_name = file.path.rsplit('/').next().unwrap_or_default().to_string();
        let candidates = bundle::find_by_name(&folder, &file_name);
        let Some(found) = bundle::best_candidate(candidates, file.size, file.sha256.as_deref()) else {
            continue;
        };
        let result = if is_video { crate::video::relink(open, &id, &found) } else { relink(open, &id, &found) };
        match result {
            Ok(()) => fixed += 1,
            Err(e) => problems.push(e),
        }
    }

    let still = s.open_ref()?.missing.len();
    problems.push(format!("Relinked {fixed} file(s); {still} still missing."));
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

/// Sets a category volume of the top-most enabled scene (decision 0028), or the project's
/// own when no scene is enabled. A scene without volumes of its own gets a copy of the
/// project's first, so only the one category changes.
///
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
    let open = s.open_mut()?;
    let volume = volume.clamp(0.0, 1.0);
    let defaults = open.project.default_volumes();
    // `open.mix` is up to date: it's recomputed after every change.
    let top = open.mix.volumes_scene.clone();
    match top.and_then(|id| open.project.scenes.iter_mut().find(|sc| sc.id == id)) {
        Some(scene) => {
            // `get_or_insert` fills the `Option` if it's `None` and returns a mutable
            // reference to what's inside either way.
            *scene.volumes.get_or_insert(defaults).get_mut(category) = volume;
        }
        None => open.project.categories.get_mut(category).volume = volume,
    }
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
) -> Result<Vec<String>, String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut problems = Vec::new();
    let mut added = Vec::new();
    let folder = folder.unwrap_or_default();

    for path in paths {
        match storage::copy_audio_into_project(&open.dir, category, &folder, &path) {
            Ok(file) => {
                let name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Sound".into());
                let sound = Sound::new(category, name, file);
                added.push(sound.id.clone());
                open.project.sounds.push(sound);
            }
            Err(e) => problems.push(e),
        }
    }

    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(added)
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
        existing.normalize = sound.normalize;
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

/// Duplicates a sound: its file is copied beside the original as "<name> (copy)" and the copy
/// joins the library right after it, with the same settings, but in no scene (decision 0043).
/// Returns the copy's id.
///
/// `async` and the copy made without holding the state lock: a big file takes a while, and the
/// app (including playback commands) would stall meanwhile.
#[tauri::command(async)]
pub fn duplicate_sound(id: String, app: AppHandle, state: AppStateRef) -> Result<String, String> {
    let (dir, sound) = {
        let s = lock(&state)?;
        let open = s.open_ref()?;
        let sound = open.project.sound(&id).ok_or("that sound no longer exists")?.clone();
        if open.missing.contains(&id) {
            return Err(format!("“{}” can't be duplicated: its file is missing", sound.name));
        }
        (open.dir.clone(), sound)
    };
    let file = storage::copy_beside(&dir, &sound.file)?;

    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    if open.dir != dir {
        // Another project was opened during the copy: don't leave a stray file in this one.
        let _ = std::fs::remove_file(storage::absolute_audio_path(&dir, &file));
        return Err("the project was closed while the sound was being copied".into());
    }
    let mut copy = sound.clone();
    copy.id = new_id("snd");
    copy.name = format!("{} (copy)", sound.name);
    copy.file = file;
    copy.last_used = None;
    let copy_id = copy.id.clone();
    let index = open.project.sounds.iter().position(|x| x.id == id).map_or(open.project.sounds.len(), |i| i + 1);
    open.project.sounds.insert(index, copy);
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(copy_id)
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

// ---- Waveforms, range previews and cropping (decision 0020) ---------------------------------

/// Plays part of a sound (crop editor). `seam` plays across the loop point.
#[tauri::command]
pub fn preview_range(
    id: String,
    start_ms: u64,
    end_ms: u64,
    seam: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let mut s = lock(&state)?;
    let state_ref = &mut *s;
    let open = state_ref.open.as_ref().ok_or("no project is open")?;
    let sound = open.project.sound(&id).ok_or("that sound no longer exists")?;
    state_ref.engine.preview_range(&open.project, &open.dir, sound, start_ms, end_ms, seam)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// The previewed sound and where it is (ms), for the waveform cursor. Polled by the UI.
#[tauri::command]
pub fn preview_position(state: AppStateRef) -> Result<Option<(String, u64)>, String> {
    Ok(lock(&state)?.engine.preview_position())
}

/// Lets the UI write to the log file (errors it shows, for example).
#[tauri::command]
pub fn log_message(level: String, message: String) {
    match level.as_str() {
        "error" => crate::logging::error(&message),
        "warn" => crate::logging::warn(&message),
        _ => crate::logging::info(&message),
    }
}

/// "See logs": opens, in the system's own app, `which` of the logs (see logging.rs):
/// `"events"` (comilito.log: errors and background events), `"errors"` (stderr.log: what
/// Comilito, GTK and WebKit printed, crashes included) or `"folder"` (the logs folder).
#[tauri::command]
pub fn open_log_file(which: String, app: AppHandle) -> Result<(), String> {
    let events = crate::logging::path().ok_or("logging isn't available")?;
    let dir = events.parent().ok_or("logging isn't available")?.to_path_buf();
    let path = match which.as_str() {
        "events" => events,
        "folder" => dir,
        "errors" => {
            let path = dir.join("stderr.log");
            if !path.exists() {
                // Only written when nobody is watching stderr (see logging::capture_stderr).
                return Err("There's no error log yet. It's written when Comilito is started without a terminal \
                            (from the desktop or the AppImage), and only on Linux."
                    .into());
            }
            path
        }
        _ => return Err(format!("unknown log \"{which}\"")),
    };
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("could not open the log: {e}"))
}

/// Removes everything Comilito stored on this computer apart from projects: preferences,
/// user themes, downloaded tools, logs, caches, the window's saved state and, unless
/// `keep_backups`, the backups (which live in the app data folder, decision 0026). Then quits.
/// Projects (in the projects folder) are left alone.
#[tauri::command]
pub fn forget_all_data(keep_backups: bool, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    use tauri::Manager;
    let backups_dir = {
        let mut s = lock(&state)?;
        s.close_project()?;
        s.backups_dir.clone()
    };
    let keep = keep_backups.then_some(backups_dir.as_path());
    let paths = app.path();
    let dirs = [
        lock(&state)?.config_dir.clone(),
        paths.app_config_dir().map_err(|e| e.to_string())?,
        paths.app_data_dir().map_err(|e| e.to_string())?,
        paths.app_local_data_dir().map_err(|e| e.to_string())?,
        paths.app_cache_dir().map_err(|e| e.to_string())?,
    ];
    for dir in dirs {
        remove_all_except(&dir, keep)?;
    }
    // Not `app.exit(0)`: Tauri's normal exit lets plugins save their state, and the
    // window-state plugin would write `.window-state.json` straight back into a folder we just
    // removed. The project is already closed and saved, so nothing else needs to run.
    std::process::exit(0);
}

/// Deletes `dir` and everything in it, except `keep` (and the folders leading to it) when
/// `keep` is inside `dir`.
fn remove_all_except(dir: &Path, keep: Option<&Path>) -> Result<(), String> {
    let err = |p: &Path, e: std::io::Error| format!("could not remove {}: {e}", p.display());
    if !dir.exists() {
        return Ok(());
    }
    match keep {
        Some(keep) if keep == dir => Ok(()),
        Some(keep) if keep.starts_with(dir) => {
            for entry in std::fs::read_dir(dir).map_err(|e| err(dir, e))?.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    remove_all_except(&path, Some(keep))?;
                } else {
                    std::fs::remove_file(&path).map_err(|e| err(&path, e))?;
                }
            }
            Ok(())
        }
        _ => std::fs::remove_dir_all(dir).map_err(|e| err(dir, e)),
    }
}

#[tauri::command]
pub fn stop_preview(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.engine.stop_preview();
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Waveform peaks of a sound between two times, shrunk to `buckets` values.
/// `async`: decoding a long file takes a while, so Tauri runs this off the main thread,
/// and the state is only locked briefly before and after.
#[tauri::command(async)]
pub fn sound_waveform(
    id: String,
    start_ms: Option<u64>,
    end_ms: Option<u64>,
    buckets: usize,
    state: AppStateRef,
) -> Result<crate::waveform::WaveformView, String> {
    let (path, key, cached) = {
        let s = lock(&state)?;
        let open = s.open_ref()?;
        let sound = open.project.sound(&id).ok_or("that sound no longer exists")?;
        let path = storage::absolute_audio_path(&open.dir, &sound.file);
        let key = format!("{}|{}", path.display(), sound.file.modified);
        let cached = s.waveforms.iter().find(|(k, _)| *k == key).map(|(_, p)| p.clone());
        (path, key, cached)
    };
    let peaks = match cached {
        Some(p) => p,
        None => {
            let computed = std::sync::Arc::new(crate::waveform::compute(&path)?);
            let mut s = lock(&state)?;
            s.waveforms.push((key, computed.clone()));
            // Keep only the few most recent: each is a few MB for long files.
            if s.waveforms.len() > 6 {
                s.waveforms.remove(0);
            }
            computed
        }
    };
    Ok(peaks.view(start_ms.unwrap_or(0), end_ms.unwrap_or(u64::MAX), buckets))
}

/// Crops a sound with ffmpeg, saving a copy (default) or overwriting the original, whose
/// file then goes to the system trash. Returns the id of the cropped sound.
#[tauri::command(async)]
pub fn crop_sound(
    id: String,
    options: crate::crop::CutOptions,
    overwrite: bool,
    app: AppHandle,
    state: AppStateRef,
) -> Result<String, String> {
    let (ffmpeg, input, sound) = {
        let s = lock(&state)?;
        let open = s.open_ref()?;
        let sound = open.project.sound(&id).ok_or("that sound no longer exists")?.clone();
        let ffmpeg = crate::tools::ffmpeg_path(&s.tools_dir).ok_or("TOOLS_MISSING")?;
        (ffmpeg, storage::absolute_audio_path(&open.dir, &sound.file), sound)
    };
    let folder = input.parent().ok_or("the sound has no folder")?.to_path_buf();
    let stem = input.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = input.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    // Hidden name, so the folder watcher never mistakes the half-written file for a new sound.
    let tmp = folder.join(format!(".{stem}.crop-tmp.{ext}"));
    if let Err(e) = crate::crop::cut(&ffmpeg, &input, &tmp, options) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    let mut s = lock(&state)?;
    s.engine.stop_preview();
    s.engine.clear_cache();
    s.waveforms.clear();
    let open = s.open_mut()?;
    let result_id = if overwrite {
        trash::delete(&input).map_err(|e| format!("could not move the original to the trash: {e}"))?;
        std::fs::rename(&tmp, &input).map_err(|e| format!("could not replace the file: {e}"))?;
        let relative = input.strip_prefix(&open.dir).map_err(|e| e.to_string())?.to_path_buf();
        let info = storage::file_info(&open.dir, &relative)?;
        if let Some(existing) = open.project.sound_mut(&id) {
            existing.file = info;
        }
        open.analysis_failed.retain(|m| m != &id);
        id
    } else {
        let target = storage::unique_path(&folder, std::path::Path::new(&format!("{stem} (cropped).{ext}")));
        std::fs::rename(&tmp, &target).map_err(|e| format!("could not save the cropped file: {e}"))?;
        let relative = target.strip_prefix(&open.dir).map_err(|e| e.to_string())?.to_path_buf();
        let info = storage::file_info(&open.dir, &relative)?;
        let mut copy = sound.clone();
        copy.id = new_id("snd");
        copy.name = format!("{} (cropped)", sound.name);
        copy.file = info;
        copy.last_used = None;
        let new_id = copy.id.clone();
        let index = open.project.sounds.iter().position(|x| x.id == id).map_or(open.project.sounds.len(), |i| i + 1);
        open.project.sounds.insert(index, copy);
        new_id
    };
    let problems = s.project_changed(Duration::from_millis(300));
    emit_state(&app, &s, problems);
    Ok(result_id)
}

// ---- Tools and downloads (decision 0019) ----------------------------------------------------

#[tauri::command(async)]
pub fn tools_status(state: AppStateRef) -> Result<crate::tools::ToolsStatus, String> {
    let dir = lock(&state)?.tools_dir.clone();
    Ok(crate::tools::status(&dir))
}

/// Downloads (or updates) yt-dlp and ffmpeg, reporting `tools-progress` events. ffmpeg is
/// replaced when it's from before decision 0051 or a newer release is out.
#[tauri::command(async)]
pub fn install_tools(app: AppHandle, state: AppStateRef) -> Result<crate::tools::ToolsStatus, String> {
    let dir = lock(&state)?.tools_dir.clone();
    let mut report = |p: crate::tools::InstallProgress| {
        let _ = app.emit("tools-progress", &p);
    };
    crate::tools::install_yt_dlp(&dir, &mut report)?;
    if crate::tools::ffmpeg_needs_install(&dir) {
        crate::tools::install_ffmpeg(&dir, &mut report)?;
        // A new ffmpeg may offer other GPU encoders (decision 0051): look again when next asked.
        lock(&state)?.video_encoders = None;
    }
    Ok(crate::tools::status(&dir))
}

#[tauri::command(async)]
pub fn tools_update_available(state: AppStateRef) -> Result<Option<bool>, String> {
    let dir = lock(&state)?.tools_dir.clone();
    Ok(crate::tools::yt_dlp_update_available(&dir))
}

#[tauri::command(async)]
pub fn remove_tools(state: AppStateRef) -> Result<(), String> {
    let dir = lock(&state)?.tools_dir.clone();
    crate::tools::remove_all(&dir)
}

/// Looks at a URL for the "Add audio from a URL" dialog.
#[tauri::command(async)]
pub fn probe_url(url: String, state: AppStateRef) -> Result<crate::downloads::Probe, String> {
    let dir = lock(&state)?.tools_dir.clone();
    crate::downloads::probe(&dir, &url)
}

#[tauri::command]
pub fn start_download(
    request: crate::downloads::DownloadRequest,
    app: AppHandle,
    state: AppStateRef,
) -> Result<String, String> {
    if request.name.trim().is_empty() {
        return Err("the sound needs a name".into());
    }
    let mut s = lock(&state)?;
    let id = crate::downloads::enqueue(&mut s, request)?;
    crate::downloads::emit_downloads(&app, &s);
    Ok(id)
}

#[tauri::command]
pub fn cancel_download(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    use crate::downloads::JobStatus;
    let mut s = lock(&state)?;
    if let Some(job) = s.downloads.iter_mut().find(|j| j.id == id)
        && matches!(job.status, JobStatus::Queued | JobStatus::Running)
    {
        job.status = JobStatus::Cancelled;
    }
    crate::downloads::save_unfinished(&s);
    crate::downloads::emit_downloads(&app, &s);
    Ok(())
}

/// Removes finished, failed and cancelled downloads from the list.
#[tauri::command]
pub fn clear_downloads(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    use crate::downloads::JobStatus;
    let mut s = lock(&state)?;
    s.downloads.retain(|j| matches!(j.status, JobStatus::Queued | JobStatus::Running));
    crate::downloads::emit_downloads(&app, &s);
    Ok(())
}

#[tauri::command]
pub fn list_downloads(state: AppStateRef) -> Result<Vec<crate::downloads::DownloadJob>, String> {
    Ok(lock(&state)?.downloads.clone())
}

/// A file dropped onto the window from the file manager (TODO.md), on webviews that give us the
/// file's contents, not its path, so they arrive as the raw request body; the name, category
/// and folder come in headers (URL-encoded). Saves a copy into the project and returns the
/// new sound's id.
#[tauri::command]
pub fn import_dropped_file(
    request: tauri::ipc::Request<'_>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<String, String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected the file's contents".into());
    };
    let header = |key: &str| {
        request
            .headers()
            .get(key)
            .and_then(|v| v.to_str().ok())
            .map(crate::downloads::percent_decode)
            .unwrap_or_default()
    };
    let name = header("x-name");
    let folder = header("x-folder");
    let shelf = match header("x-category").as_str() {
        "music" => Shelf::Music,
        "effects" => Shelf::Effects,
        "ambienceVideo" => Shelf::AmbienceVideo,
        "videoEffect" => Shelf::VideoEffect,
        _ => Shelf::Ambience,
    };

    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let info = storage::save_dropped_file(&open.dir, shelf, &folder, &name, bytes)?;
    let stem = std::path::Path::new(&name)
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Sound".into());
    let id = match shelf.video_kind() {
        Some(kind) => {
            let video = crate::video::new_video(stem, info, kind);
            let id = video.id.clone();
            open.project.videos.push(video);
            id
        }
        None => {
            let category = match shelf {
                Shelf::Music => Category::Music,
                Shelf::Effects => Category::Effects,
                _ => Category::Ambience,
            };
            let sound = Sound::new(category, stem, info);
            let id = sound.id.clone();
            open.project.sounds.push(sound);
            id
        }
    };
    open.folders = storage::list_folders(&open.dir);
    open.video_folders = crate::video::list_folders(&open.dir);
    // `open` isn't used after this point, so `s` can be borrowed again.
    if shelf.video_kind().is_some() {
        crate::video_convert::note_new(&mut s, std::slice::from_ref(&id), crate::video_convert::Origin::Added);
    }
    crate::logging::info(&format!("Imported dropped file \"{name}\""));
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(id)
}

// ---- Library folders ------------------------------------------------------------------------

#[tauri::command]
pub fn create_folder(category: Shelf, folder: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    storage::create_folder(&open.dir, category, folder.trim())?;
    open.folders = storage::list_folders(&open.dir);
    open.video_folders = crate::video::list_folders(&open.dir);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Renames a Library folder and points every sound (or video) inside it at its new path.
/// `category` is a shelf: an audio category, or "video".
#[tauri::command]
pub fn rename_folder(
    category: Shelf,
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
    let old_prefix = format!("{}/{}/", category.root(), from);
    let new_root = storage::shelf_folder_path(&open.dir, category, &to);
    let new_prefix = format!(
        "{}/",
        storage::to_json_path(new_root.strip_prefix(&open.dir).unwrap_or(&new_root))
    );
    // `iter_mut().map(...)` over both lists: each item's `file` is borrowed mutably in turn.
    let files = open
        .project
        .sounds
        .iter_mut()
        .map(|s| &mut s.file)
        .chain(open.project.videos.iter_mut().map(|v| &mut v.file));
    for file in files {
        if let Some(rest) = file.path.strip_prefix(&old_prefix) {
            file.path = format!("{new_prefix}{rest}");
        }
    }
    open.folders = storage::list_folders(&open.dir);
    open.video_folders = crate::video::list_folders(&open.dir);
    s.engine.clear_cache();
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(())
}

#[tauri::command]
pub fn delete_folder(category: Shelf, folder: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    storage::delete_empty_folder(&open.dir, category, &folder)?;
    open.folders = storage::list_folders(&open.dir);
    open.video_folders = crate::video::list_folders(&open.dir);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Opens a Library folder (or the whole `audio/` folder) in the system file manager.
#[tauri::command]
pub fn open_audio_folder(
    category: Option<Shelf>,
    folder: Option<String>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<(), String> {
    let s = lock(&state)?;
    let dir = &s.open_ref()?.dir;
    let path = match category {
        Some(c) => storage::shelf_folder_path(dir, c, folder.as_deref().unwrap_or("")),
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
        ducks_all: false,
        volumes: None,
        video: None,
        hides_video: false,
        video_effects: Vec::new(),
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

/// Scene groups have no list of their own: a group is just the `group` name its scenes share.
/// So renaming one renames it on every scene in it, and renaming it to an existing group's
/// name merges the two (unlike preset groups, which can be empty and so are stored).
#[tauri::command]
pub fn scene_group_rename(from: String, to: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let to = to.trim().to_string();
    if to.is_empty() {
        return Err("the group needs a name".into());
    }
    let mut s = lock(&state)?;
    for scene in &mut s.open_mut()?.project.scenes {
        if scene.group.as_deref() == Some(from.as_str()) {
            scene.group = Some(to.clone());
        }
    }
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Moves one scene into a group (or out of any group, with `None`).
#[tauri::command]
pub fn scene_set_group(scene_id: String, group: Option<String>, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let group = group.map(|g| g.trim().to_string()).filter(|g| !g.is_empty());
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let scene = open
        .project
        .scenes
        .iter_mut()
        .find(|sc| sc.id == scene_id)
        .ok_or("that scene no longer exists")?;
    scene.group = group;
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// "Ungroup": the group's scenes are kept, without a group.
#[tauri::command]
pub fn scene_group_ungroup(name: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    for scene in &mut s.open_mut()?.project.scenes {
        if scene.group.as_deref() == Some(name.as_str()) {
            scene.group = None;
        }
    }
    s.project_changed(Duration::ZERO);
    emit_state(&app, &s, Vec::new());
    Ok(())
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
                    role: Role::Foreground,
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

/// Removes every sound and video whose file is missing (from the library and all scenes).
/// For when files were deleted on purpose outside the app.
#[tauri::command]
pub fn remove_missing_sounds(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let missing = std::mem::take(&mut open.missing);
    open.project.sounds.retain(|snd| !missing.contains(&snd.id));
    open.project.videos.retain(|v| !missing.contains(&v.id));
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

/// The role picker in an expanded stack entry: changes that layer's role in its scene.
#[tauri::command]
pub fn set_layer_role(
    scene_id: String,
    layer_index: usize,
    role: Role,
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
    layer.role = role;
    // A role changes which scenes duck the layer, which can be a big jump in volume: fade it
    // like a scene edit (the scene's transition, at most half a second).
    let transition = millis(scene.transition_ms.unwrap_or(open.project.defaults.transition_ms));
    let problems = s.project_changed(transition.min(Duration::from_millis(500)));
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
    // Esc blanks the video window too (decision 0035), video effect included.
    s.video_effect = None;
    emit_state(&app, &s, Vec::new());
    Ok(())
}
