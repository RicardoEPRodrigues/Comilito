mod audio;
mod commands;
mod mix;
mod prefs;
mod project;
mod state;
mod storage;

use std::{
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use audio::AudioEngine;
use state::{AppState, SAVE_DELAY, emit_state};
use tauri::{AppHandle, Manager, RunEvent};

// On Android this function becomes the app's entry point; on desktop main.rs calls it.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // `setup` runs once the app exists, so we can ask Tauri for OS folders.
        .setup(|app| {
            // Without audio output the app is useless, so refuse to start with a clear message.
            let engine = AudioEngine::new()?;

            let config_dir = app.path().config_dir()?.join("comilito");
            let default_projects_root = app.path().document_dir()?.join("Comilito");
            let prefs = prefs::load(&config_dir);

            // Hands ownership of the state to Tauri, which shares it with every command.
            app.manage(Mutex::new(AppState {
                engine,
                prefs,
                config_dir,
                default_projects_root,
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
            commands::keep_my_version,
            commands::rename_project,
            commands::set_master_volume,
            commands::set_category_volume,
            commands::set_project_defaults,
            commands::set_backup_settings,
            commands::reset_project_defaults,
            commands::add_sounds,
            commands::update_sound,
            commands::remove_sound,
            commands::preview_sound,
            commands::play_effect,
            commands::create_scene,
            commands::update_scene,
            commands::duplicate_scene,
            commands::delete_scene,
            commands::set_layer_volume,
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

/// Runs forever on its own thread, a few times per second:
/// - moves music playlists to the next track when one ends,
/// - saves the project a moment after the last change,
/// - notices when comilito.json was changed by another program,
/// - tells the UI when what's audible changed.
fn ticker(app: AppHandle) {
    const TICK: Duration = Duration::from_millis(250);
    const DISK_CHECK: Duration = Duration::from_secs(2);
    let mut last_disk_check = Instant::now();
    let mut last_info = None;

    loop {
        thread::sleep(TICK);
        let state = app.state::<Mutex<AppState>>();
        let Ok(mut s) = state.lock() else {
            return;
        };
        let mut problems = Vec::new();
        let mut changed = false;

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
        }

        let info = s.snapshot().playback;
        if last_info.as_ref() != Some(&info) {
            last_info = Some(info);
            changed = true;
        }
        if changed || !problems.is_empty() {
            emit_state(&app, &s, problems);
        }
    }
}
