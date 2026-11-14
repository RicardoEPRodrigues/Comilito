//! Registers Comilito with the OS as a media player (decision 0030), so the desktop's media
//! controls (GNOME/KDE's player widget, media keys, headset buttons, Windows' volume flyout)
//! can play, pause and stop the tapestry, and show what's playing.
//!
//! The `souvlaki` crate speaks each OS's protocol: MPRIS over D-Bus on Linux, the System Media
//! Transport Controls on Windows, Now Playing on macOS. This module only translates between it
//! and the app:
//!
//! - OS → app: Play, Pause, Toggle and Stop run the same code as Space and Esc.
//! - app → OS: a thread looks at the state four times a second and reports changes in the
//!   transport (playing / paused / stopped) and in the title (top scene, music, project).

use std::{
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use tauri::{AppHandle, Manager};

use crate::{
    audio::Transport,
    logging,
    state::{AppState, emit_state},
};

/// What the OS is shown. Compared with the last one sent, so the OS is only told about changes.
#[derive(PartialEq, Clone)]
struct NowPlaying {
    transport: Transport,
    /// The top scene of the tapestry, or the project's name when no scene is enabled.
    title: String,
    /// The music track playing, if any.
    artist: Option<String>,
    /// The project.
    album: Option<String>,
}

/// Starts the media controls on their own thread. If the OS has none (no D-Bus session, say),
/// it's logged and Comilito works as before.
pub fn start(app: &AppHandle) {
    // On Windows the controls belong to a window. Its handle (HWND) is a raw pointer, which
    // Rust won't send to another thread, so it travels as a plain number.
    #[cfg(target_os = "windows")]
    let hwnd = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as usize);
    #[cfg(not(target_os = "windows"))]
    let hwnd: Option<usize> = None;

    let app = app.clone();
    thread::spawn(move || {
        let config = PlatformConfig {
            display_name: "Comilito",
            dbus_name: "comilito",
            hwnd: hwnd.map(|h| h as *mut std::ffi::c_void),
        };
        let mut controls = match MediaControls::new(config) {
            Ok(c) => c,
            Err(e) => {
                logging::warn(&format!("media controls unavailable: {e:?}"));
                return;
            }
        };

        // The handler runs on souvlaki's own thread, so it gets its own handle to the app.
        let events_app = app.clone();
        if let Err(e) = controls.attach(move |event| handle_event(&events_app, event)) {
            logging::warn(&format!("media controls unavailable: {e:?}"));
            return;
        }

        report_changes(&app, &mut controls);
    });
}

/// A button pressed in the OS's media controls.
fn handle_event(app: &AppHandle, event: MediaControlEvent) {
    let state = app.state::<Mutex<AppState>>();
    let Ok(mut s) = state.lock() else {
        return;
    };
    let playing = s.engine.playback_transport() == Transport::Playing;
    match event {
        // Play and Pause only act when they'd change something; Toggle is Space.
        MediaControlEvent::Play if !playing => toggle_play(app, &mut s),
        MediaControlEvent::Pause if playing => toggle_play(app, &mut s),
        MediaControlEvent::Toggle => toggle_play(app, &mut s),
        MediaControlEvent::Stop => {
            let fade = Duration::from_millis(s.prefs.playback.stop_fade_ms as u64);
            s.engine.stop_all(fade);
            emit_state(app, &s, Vec::new());
        }
        MediaControlEvent::Raise => {
            drop(s); // Showing the window doesn't need the state; don't hold the lock.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        // Next/previous, seeking, volume and quitting aren't offered: a tapestry has no track
        // list, and the volumes are the DM's to set.
        _ => {}
    }
}

/// Space, as in `commands::toggle_play`, for whoever already holds the lock.
fn toggle_play(app: &AppHandle, s: &mut AppState) {
    let fade = Duration::from_millis(s.prefs.playback.pause_fade_ms as u64);
    let Some(open) = &s.open else {
        return;
    };
    let problems = s.engine.toggle_play(&open.project, &open.dir, &open.mix, fade);
    emit_state(app, s, problems);
}

/// Runs forever: tells the OS whenever what it shows has changed.
fn report_changes(app: &AppHandle, controls: &mut MediaControls) {
    const TICK: Duration = Duration::from_millis(250);
    let mut last: Option<NowPlaying> = None;
    let mut last_error = Instant::now() - Duration::from_secs(3600);

    loop {
        thread::sleep(TICK);
        let now = {
            let state = app.state::<Mutex<AppState>>();
            let Ok(s) = state.lock() else {
                return;
            };
            now_playing(&s)
        };
        if last.as_ref() == Some(&now) {
            continue;
        }

        let playback = match now.transport {
            Transport::Playing => MediaPlayback::Playing { progress: None },
            Transport::Paused => MediaPlayback::Paused { progress: None },
            Transport::Stopped => MediaPlayback::Stopped,
        };
        let metadata = MediaMetadata {
            title: Some(&now.title),
            artist: now.artist.as_deref(),
            album: now.album.as_deref(),
            ..Default::default()
        };
        let result = controls.set_metadata(metadata).and_then(|_| controls.set_playback(playback));
        match result {
            Ok(()) => last = Some(now),
            // Try again next tick, but log at most once a minute.
            Err(e) if last_error.elapsed() > Duration::from_secs(60) => {
                last_error = Instant::now();
                logging::warn(&format!("could not update the media controls: {e:?}"));
            }
            Err(_) => {}
        }
    }
}

fn now_playing(s: &AppState) -> NowPlaying {
    let Some(open) = &s.open else {
        return NowPlaying {
            transport: Transport::Stopped,
            title: "Comilito".into(),
            artist: None,
            album: None,
        };
    };
    let project = &open.project;
    let top_scene = open
        .mix
        .volumes_scene
        .as_ref()
        .and_then(|id| project.scenes.iter().find(|scene| &scene.id == id));
    let info = s.engine.playback_info(&open.mix);
    NowPlaying {
        transport: info.transport,
        title: top_scene.map_or_else(|| project.name.clone(), |scene| scene.name.clone()),
        artist: info
            .music_sound
            .as_deref()
            .and_then(|id| project.sound(id))
            .map(|sound| sound.name.clone()),
        album: Some(project.name.clone()),
    }
}
