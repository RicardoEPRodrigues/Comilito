mod audio;
mod bundle;
mod commands;
mod loudness;
mod mix;
mod prefs;
mod project;
mod sample;
mod state;
mod storage;

use std::{
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use audio::AudioEngine;
use state::{AppState, SAVE_DELAY, emit_state};
use tauri::{AppHandle, Emitter, Manager, RunEvent, path::BaseDirectory};

// On Android this function becomes the app's entry point; on desktop main.rs calls it.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Opens web links (About page) in the user's browser instead of inside the app.
        .plugin(tauri_plugin_opener::init())
        // `setup` runs once the app exists, so we can ask Tauri for OS folders.
        .setup(|app| {
            // Without audio output the app is useless, so refuse to start with a clear message.
            let engine = AudioEngine::new()?;

            let config_dir = app.path().config_dir()?.join("comilito");
            let default_projects_root = app.path().document_dir()?.join("Comilito");
            let prefs = prefs::load(&config_dir);
            // Where the bundled sample audio was installed (`bundle.resources` in
            // tauri.conf.json). In development that's next to the debug binary.
            let sample_dir = app
                .path()
                .resolve("sample", BaseDirectory::Resource)
                .ok()
                .filter(|p| p.is_dir());

            // Hands ownership of the state to Tauri, which shares it with every command.
            app.manage(Mutex::new(AppState {
                engine,
                prefs,
                config_dir,
                default_projects_root,
                sample_dir,
                open: None,
            }));

            // `app.handle().clone()` gives the thread its own handle to the app. The thread
            // outlives this function, so it can't borrow anything from here: `move` makes the
            // closure take ownership of what it uses.
            let handle = app.handle().clone();
            thread::spawn(move || ticker(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::get_state,
            commands::get_preferences,
            commands::set_preferences,
            commands::list_projects,
            commands::projects_folder,
            commands::create_project,
            commands::open_project,
            commands::close_project,
            commands::reload_project,
            commands::delete_project,
            commands::keep_my_version,
            commands::rename_project,
            commands::export_project,
            commands::import_project,
            commands::backups_folder,
            commands::relink_from_folder,
            commands::relink_sound,
            commands::set_master_volume,
            commands::set_category_volume,
            commands::set_project_defaults,
            commands::set_backup_settings,
            commands::reset_project_defaults,
            commands::add_sounds,
            commands::update_sound,
            commands::remove_sound,
            commands::preview_sound,
            commands::toggle_effect,
            commands::set_sound_category,
            commands::create_scene,
            commands::update_scene,
            commands::duplicate_scene,
            commands::delete_scene,
            commands::set_layer_volume,
            commands::add_sound_to_scene,
            commands::stack_insert,
            commands::stack_move,
            commands::stack_remove,
            commands::stack_set_enabled,
            commands::stack_switch,
            commands::scene_hotkey,
            commands::preset_save,
            commands::preset_load,
            commands::preset_insert,
            commands::preset_rename,
            commands::preset_delete,
            commands::toggle_play,
            commands::stop_all,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // `run` takes a closure that sees every app event. We only care about exiting:
    // save the project so the last changes aren't lost.
    app.run(|app, event| {
        if let RunEvent::Exit = event {
            let state = app.state::<Mutex<AppState>>();
            if let Ok(mut s) = state.lock()
                && let Err(e) = s.close_project()
            {
                eprintln!("could not save on exit: {e}");
            }
        }
    });
}

/// Slow work the ticker found to do. It's done *after* unlocking the state, so a big
/// backup or hash never freezes the UI or the commands.
enum SlowJob {
    Backup {
        dir: PathBuf,
        project: Box<project::Project>,
        backups_dir: PathBuf,
    },
    Hash {
        sound_id: String,
        path: PathBuf,
        size: u64,
    },
    /// Measure loudness for normalization (see loudness.rs).
    Analyze {
        sound_id: String,
        path: PathBuf,
    },
}

/// Runs forever on its own thread, a few times per second:
/// - moves music playlists to the next track when one ends,
/// - saves the project a moment after the last change,
/// - notices when comilito.json was changed by another program,
/// - makes backups every N minutes and hashes audio files, one at a time, off the lock,
/// - tells the UI when what's audible changed.
fn ticker(app: AppHandle) {
    const TICK: Duration = Duration::from_millis(250);
    const DISK_CHECK: Duration = Duration::from_secs(2);
    let mut last_disk_check = Instant::now();
    let mut last_info = None;
    let mut last_hashing_remaining = 0;
    let mut last_analyzing_remaining = 0;

    loop {
        thread::sleep(TICK);
        let state = app.state::<Mutex<AppState>>();

        // ---- Quick work, with the state locked ----
        let job = {
            let Ok(mut s) = state.lock() else {
                return;
            };
            let mut problems = Vec::new();
            let mut changed = false;
            let mut job = None;
            let backups_dir = s.backups_root();

            // Borrow the two fields separately (see `AppState::project_changed`).
            let s_ref = &mut *s;
            if let Some(open) = &mut s_ref.open {
                problems.extend(s_ref.engine.tick(&open.project, &open.dir, &open.mix));

                if open.dirty_since.is_some_and(|t| t.elapsed() >= SAVE_DELAY) {
                    if let Err(e) = storage::save_project(&open.dir, &open.project) {
                        problems.push(format!("could not save: {e}"));
                    }
                    open.dirty_since = None;
                    open.known_file_time = storage::project_file_modified(&open.dir);
                }

                if last_disk_check.elapsed() >= DISK_CHECK {
                    last_disk_check = Instant::now();
                    let on_disk = storage::project_file_modified(&open.dir);
                    if on_disk != open.known_file_time && !open.changed_on_disk {
                        open.changed_on_disk = true;
                        changed = true;
                    }
                }

                let interval = Duration::from_secs(open.project.backups.interval_minutes.max(1) as u64 * 60);
                if open.changed_since_backup && open.last_backup.elapsed() >= interval {
                    open.changed_since_backup = false;
                    open.last_backup = Instant::now();
                    job = Some(SlowJob::Backup {
                        dir: open.dir.clone(),
                        project: Box::new(open.project.clone()),
                        backups_dir,
                    });
                } else if let Some(sound) = open.next_to_analyze() {
                    // Loudness before hashes: it changes how things sound, hashes don't.
                    job = Some(SlowJob::Analyze {
                        sound_id: sound.id.clone(),
                        path: storage::absolute_audio_path(&open.dir, &sound.file),
                    });
                } else if let Some(sound) = open
                    .project
                    .sounds
                    .iter()
                    .find(|snd| snd.file.sha256.is_none() && !open.missing.contains(&snd.id))
                {
                    job = Some(SlowJob::Hash {
                        sound_id: sound.id.clone(),
                        path: storage::absolute_audio_path(&open.dir, &sound.file),
                        size: sound.file.size,
                    });
                }
            }

            let snapshot = s.snapshot();
            if last_info.as_ref() != Some(&snapshot.playback)
                || last_hashing_remaining != snapshot.hashing_remaining
                || last_analyzing_remaining != snapshot.analyzing_remaining
            {
                last_info = Some(snapshot.playback);
                last_hashing_remaining = snapshot.hashing_remaining;
                last_analyzing_remaining = snapshot.analyzing_remaining;
                changed = true;
            }
            if changed || !problems.is_empty() {
                emit_state(&app, &s, problems);
            }
            job
            // The lock guard `s` is dropped here, at the end of the block.
        };

        // ---- Slow work, with the state unlocked ----
        match job {
            Some(SlowJob::Backup { dir, project, backups_dir }) => {
                if let Err(e) = bundle::write_backup(&dir, &project, &backups_dir) {
                    let _ = app.emit("notice", format!("Backup failed: {e}"));
                }
            }
            Some(SlowJob::Hash { sound_id, path, size }) => {
                let result = bundle::sha256_file(&path);
                // Lock again only to store the result, and only if the sound still refers to
                // the same file (it may have been removed or relinked meanwhile).
                if let Ok(mut s) = state.lock()
                    && let Some(open) = &mut s.open
                {
                    match result {
                        Ok(hash) => {
                            if let Some(sound) = open.project.sound_mut(&sound_id)
                                && sound.file.size == size
                                && sound.file.sha256.is_none()
                            {
                                sound.file.sha256 = Some(hash);
                                open.dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                        // Unreadable: treat it as missing so we don't retry forever.
                        Err(_) => {
                            if !open.missing.contains(&sound_id) {
                                open.missing.push(sound_id);
                            }
                        }
                    }
                }
            }
            Some(SlowJob::Analyze { sound_id, path }) => {
                let result = loudness::measure(&path);
                if let Ok(mut s) = state.lock() {
                    let mut problems = Vec::new();
                    if let Some(open) = &mut s.open {
                        match result {
                            Ok(measured) => {
                                if let Some(sound) = open.project.sound_mut(&sound_id)
                                    && sound.file.loudness.is_none()
                                {
                                    sound.file.loudness = Some(measured);
                                }
                            }
                            Err(e) => {
                                open.analysis_failed.push(sound_id);
                                problems.push(format!("Couldn't measure loudness: {e}"));
                            }
                        }
                    }
                    // Re-apply the mix so a playing sound glides to its normalized volume.
                    problems.extend(s.project_changed(Duration::from_secs(1)));
                    emit_state(&app, &s, problems);
                }
            }
            None => {}
        }
    }
}
