mod audio;
mod bundle;
mod commands;
mod crop;
mod desktop;
mod downloads;
mod logging;
mod media_controls;
mod loudness;
mod mix;
mod prefs;
mod project;
mod relocate;
mod sample;
mod state;
mod storage;
mod themes;
mod tools;
mod video;
mod video_convert;
mod video_window;
mod waveform;

use std::{
    path::PathBuf,
    sync::{Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use audio::AudioEngine;
use state::{AppState, SAVE_DELAY, emit_state};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent, path::BaseDirectory};

// On Android this function becomes the app's entry point; on desktop main.rs calls it.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Before anything else, so a crash during startup leaves a trace (see logging.rs).
    #[cfg(target_os = "linux")]
    logging::capture_stderr("com.comilito.app");
    logging::log_panics();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Opens web links (About page) in the user's browser instead of inside the app.
        .plugin(tauri_plugin_opener::init())
        // Remembers the window's size, position and maximised state between launches (saved
        // when the app closes, restored when the window is created). Not visibility or
        // decorations: those always come from tauri.conf.json.
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        // `setup` runs once the app exists, so we can ask Tauri for OS folders.
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            logging::init(&data_dir.join("logs"));

            let config_dir = app.path().config_dir()?.join("comilito");
            let prefs = prefs::load(&config_dir);

            // Without audio output the app is useless, so refuse to start with a clear message
            // (in the log too: it may have been started without a terminal).
            let (engine, warning) = AudioEngine::new(prefs.playback.output_device.as_deref())
                .inspect_err(|e| logging::error(&format!("can't start: {e}")))?;
            if let Some(warning) = warning {
                logging::warn(&warning);
            }

            // Some systems don't define a Documents folder (on Linux, no XDG_DOCUMENTS_DIR in
            // user-dirs.dirs). Fall back to ~/Documents rather than refusing to start.
            // `.or_else(...)` tries the second lookup only if the first returned an error.
            let documents = app
                .path()
                .document_dir()
                .or_else(|_| app.path().home_dir().map(|home| home.join("Documents")))?;
            let default_projects_root = documents.join("Comilito");
            let backups_dir = data_dir.join("backups");
            // Older versions kept backups in `<projects folder>/Backups`. Move them in the
            // background (it may copy across disks); new backups already go to the new place.
            let projects_root = prefs.projects_folder.clone().unwrap_or_else(|| default_projects_root.clone());
            let migrate_to = backups_dir.clone();
            thread::spawn(move || match relocate::migrate_backups(&projects_root, &migrate_to) {
                Ok(0) => {}
                Ok(n) => logging::info(&format!("Moved {n} backups to {}", migrate_to.display())),
                Err(e) => logging::error(&format!("could not move the old backups: {e}")),
            });
            // Where the bundled sample audio was installed (`bundle.resources` in
            // tauri.conf.json). In development that's next to the debug binary.
            let sample_dir = app
                .path()
                .resolve("sample", BaseDirectory::Resource)
                .ok()
                .filter(|p| p.is_dir());

            // Hands ownership of the state to Tauri, which shares it with every command.
            let tools_dir = tools::tools_dir(&data_dir);
            // A channel: the sending half wakes the download worker, which owns the other half.
            let (download_wakeup, download_queue) = mpsc::channel();
            let (conversion_wakeup, conversion_queue) = mpsc::channel();
            // Downloads cut off by quitting last time; each resumes when its project opens.
            let waiting_downloads = downloads::load_saved(&data_dir, &app.path().app_cache_dir()?);
            app.manage(Mutex::new(AppState {
                engine,
                prefs,
                config_dir,
                default_projects_root,
                backups_dir: backups_dir.clone(),
                moving_projects: false,
                sample_dir,
                open: None,
                tools_dir,
                downloads: Vec::new(),
                download_wakeup: Some(download_wakeup),
                waiting_downloads,
                download_process: None,
                data_dir: data_dir.clone(),
                backing_up: false,
                quitting: false,
                conversions: Vec::new(),
                conversion_wakeup: Some(conversion_wakeup),
                conversion_process: None,
                video_offers: Default::default(),
                video_window_open: false,
                video_effect: None,
                video_positions: Default::default(),
                waveforms: Vec::new(),
            }));

            // `app.handle().clone()` gives the thread its own handle to the app. The thread
            // outlives this function, so it can't borrow anything from here: `move` makes the
            // closure take ownership of what it uses.
            let handle = app.handle().clone();
            thread::spawn(move || ticker(handle));
            let handle = app.handle().clone();
            thread::spawn(move || desktop::watch(handle));
            let handle = app.handle().clone();
            thread::spawn(move || downloads::worker(handle, download_queue));
            let handle = app.handle().clone();
            thread::spawn(move || video_convert::worker(handle, conversion_queue));
            media_controls::start(app.handle());
            Ok(())
        })
        // Closing the main window quits the app. If that would cut off work (downloads, a
        // backup...), the close is held back and the UI asks first (`confirm_quit`).
        .on_window_event(|window, event| {
            // The video window closed (its ✕, or the header button): tell the main window.
            if let WindowEvent::Destroyed = event
                && window.label() == video_window::LABEL
            {
                let state = window.state::<Mutex<AppState>>();
                if let Ok(mut s) = state.lock() {
                    s.video_window_open = false;
                    emit_state(window.app_handle(), &s, Vec::new());
                }
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                let state = window.state::<Mutex<AppState>>();
                let Ok(mut s) = state.lock() else { return };
                if s.quitting {
                    return;
                }
                let work = s.busy_work();
                if s.prefs.behaviour.warn_on_quit && !work.is_empty() {
                    api.prevent_close();
                    let _ = window.emit("quit-requested", work);
                } else {
                    s.quitting = true;
                    // The app only exits once every window is closed.
                    video_window::close(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::confirm_quit,
            video::add_videos,
            video::update_videos,
            video::remove_video,
            video::move_videos,
            video::reveal_video,
            video::relink_video,
            video::add_video_to_scene,
            video::video_streaming,
            video_convert::convert_videos,
            video_convert::cancel_conversion,
            video_convert::clear_conversions,
            video_convert::list_conversions,
            video_window::toggle_video_window,
            video_window::show_video_window,
            video_window::list_screens,
            video_window::set_video_screen,
            video_window::set_video_fullscreen,
            video_window::play_video_effect,
            video_window::video_effect_ended,
            video_window::save_video_position,
            video_window::video_positions,
            commands::app_version,
            commands::get_state,
            commands::get_preferences,
            commands::set_preferences,
            commands::list_output_devices,
            commands::desktop_appearance,
            commands::list_user_themes,
            commands::save_user_theme,
            commands::delete_user_theme,
            commands::import_theme,
            commands::export_theme,
            commands::read_theme_asset,
            commands::add_theme_file,
            commands::list_projects,
            commands::projects_folder,
            commands::duplicate_project,
            commands::show_projects_folder,
            commands::move_projects_folder,
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
            commands::import_dropped_file,
            commands::update_sounds,
            commands::remove_sound,
            commands::preview_sound,
            commands::preview_range,
            commands::preview_position,
            commands::log_message,
            commands::open_log_file,
            commands::forget_all_data,
            commands::stop_preview,
            commands::sound_waveform,
            commands::crop_sound,
            commands::tools_status,
            commands::install_tools,
            commands::tools_update_available,
            commands::remove_tools,
            commands::probe_url,
            commands::start_download,
            commands::cancel_download,
            commands::clear_downloads,
            commands::list_downloads,
            commands::toggle_effect,
            commands::move_sounds,
            commands::create_folder,
            commands::rename_folder,
            commands::delete_folder,
            commands::open_audio_folder,
            commands::reveal_sound,
            commands::create_scene,
            commands::update_scene,
            commands::duplicate_scene,
            commands::scene_group_rename,
            commands::scene_set_group,
            commands::scene_group_ungroup,
            commands::delete_scene,
            commands::set_layer_volume,
            commands::set_layer_role,
            commands::add_sound_to_scene,
            commands::remove_sound_from_scene,
            commands::remove_missing_sounds,
            commands::stack_insert,
            commands::stack_move,
            commands::stack_remove,
            commands::stack_set_enabled,
            commands::stack_switch,
            commands::scene_hotkey,
            commands::preset_save,
            commands::preset_update,
            commands::preset_set_entries,
            commands::preset_load,
            commands::preset_insert,
            commands::preset_rename,
            commands::preset_duplicate,
            commands::preset_set_group,
            commands::preset_delete,
            commands::preset_group_create,
            commands::preset_group_rename,
            commands::preset_group_delete,
            commands::stack_clear,
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
            if let Ok(mut s) = state.lock() {
                s.quitting = true;
                // A child process outlives its parent; stop a running yt-dlp (and the ffmpeg
                // it may have started) so they don't keep writing into the folder the download
                // resumes from next time.
                // The same for an ffmpeg converting a video (its working file is cleaned up the
                // next time the project opens).
                for process in [s.download_process.take(), s.conversion_process.take()].into_iter().flatten() {
                    if let Ok(mut child) = process.lock() {
                        tools::kill_tree(&mut child);
                    }
                }
                if let Err(e) = s.close_project() {
                    eprintln!("could not save on exit: {e}");
                }
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
    /// Look at a video with ffprobe (see video_convert.rs).
    ProbeVideo {
        video_id: String,
        path: PathBuf,
        ffprobe: PathBuf,
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
            let mut disk_changed = false;
            let mut new_videos: Vec<String> = Vec::new();
            let ffprobe = tools::ffprobe_path(&s.tools_dir);
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

                    // Audio files added or removed outside the app (file manager, sync tool).
                    let refresh = open.refresh_from_disk();
                    if refresh.added > 0 {
                        problems.push(format!(
                            "Added {} new sound{} found in the project folder.",
                            refresh.added,
                            if refresh.added == 1 { "" } else { "s" }
                        ));
                    }
                    if !refresh.new_videos.is_empty() {
                        let n = refresh.new_videos.len();
                        new_videos = refresh.new_videos.clone();
                        problems.push(format!(
                            "Added {n} new video{} found in the project folder.",
                            if n == 1 { "" } else { "s" }
                        ));
                    }
                    if refresh.added > 0 || !refresh.new_videos.is_empty() || refresh.missing_changed {
                        disk_changed = true;
                    }
                }

                let interval = Duration::from_secs(open.project.backups.interval_minutes.max(1) as u64 * 60);
                if open.changed_since_backup && open.last_backup.elapsed() >= interval {
                    open.changed_since_backup = false;
                    open.last_backup = Instant::now();
                    s_ref.backing_up = true;
                    job = Some(SlowJob::Backup {
                        dir: open.dir.clone(),
                        project: Box::new(open.project.clone()),
                        backups_dir,
                    });
                } else if let Some(video) = ffprobe.as_ref().and_then(|_| {
                    open.project.videos.iter().find(|v| {
                        v.info.is_none() && !open.missing.contains(&v.id) && !open.analysis_failed.contains(&v.id)
                    })
                }) {
                    // Before loudness: a new video may need converting, which takes a while.
                    job = Some(SlowJob::ProbeVideo {
                        video_id: video.id.clone(),
                        path: storage::absolute_audio_path(&open.dir, &video.file),
                        ffprobe: ffprobe.clone().unwrap_or_default(),
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

            if !new_videos.is_empty() {
                video_convert::note_new(&mut s, &new_videos, video_convert::Origin::Folder);
            }
            // Nothing left to look at: offer the videos that may not play (decision 0035).
            if !matches!(job, Some(SlowJob::ProbeVideo { .. })) {
                video_convert::flush_offer(&app, &mut s);
            }
            if disk_changed {
                // Marks the project for saving (new sounds) and re-applies the mix.
                problems.extend(s.project_changed(Duration::from_millis(500)));
                changed = true;
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
                let result = bundle::write_backup(&dir, &project, &backups_dir);
                if let Ok(mut s) = state.lock() {
                    s.backing_up = false;
                }
                match result {
                    Ok(path) => logging::info(&format!("Backup written: {}", path.display())),
                    Err(e) => {
                        logging::error(&format!("Backup failed: {e}"));
                        let _ = app.emit("notice", format!("Backup failed: {e}"));
                    }
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
                                    && sound.file.needs_analysis()
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
            Some(SlowJob::ProbeVideo { video_id, path, ffprobe }) => {
                let result = video_convert::probe(&ffprobe, &path);
                if let Ok(mut s) = state.lock() {
                    let mut problems = Vec::new();
                    match result {
                        Ok(info) => {
                            video_convert::probed(&mut s, &video_id, &info);
                            if let Some(open) = &mut s.open
                                && let Some(video) = open.project.video_mut(&video_id)
                            {
                                video.info = Some(info);
                                open.dirty_since.get_or_insert_with(Instant::now);
                            }
                        }
                        Err(e) => {
                            s.video_offers.fresh.remove(&video_id);
                            if let Some(open) = &mut s.open {
                                open.analysis_failed.push(video_id);
                            }
                            problems.push(format!("Couldn't read a video: {e}"));
                        }
                    }
                    emit_state(&app, &s, problems);
                }
            }
            None => {}
        }
    }
}
