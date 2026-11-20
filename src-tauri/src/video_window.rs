//! The video window (decision 0035): a second window that shows the top scene's videos, meant
//! for a TV or projector. It runs the same frontend as the main window; `main.ts` sees its
//! label ("video") and shows only the player (`VideoWindow.svelte`).
//!
//! Rust decides *what* should show (`TargetMix::video`, the playing video effect, the
//! transport); the window decides *how*: which video of the playlist, when to switch, the
//! crossfades. Where each video stopped is kept here (`video_positions`), so it survives the
//! window being closed and opened again while the app runs.

use std::{collections::HashMap, path::Path};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::{
    commands::{AppStateRef, lock},
    state::{AppState, emit_state},
};

pub const LABEL: &str = "video";

/// A screen the window can go to, for the picker on its top bar and in Preferences.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

/// The connected screens, in the OS's order (on Linux, GDK's monitor numbers). Screens the OS
/// doesn't name (Wayland calls a laptop's own screen "Unknown") get "Screen 1", "Screen 2"...,
/// and a name used twice gets its number too, so every name picks one screen.
fn screens(app: &AppHandle) -> Vec<(Screen, tauri::Monitor)> {
    let primary = app.primary_monitor().ok().flatten().map(|m| *m.position());
    let monitors = app.available_monitors().unwrap_or_default();
    let os_names: Vec<Option<String>> = monitors
        .iter()
        .map(|m| m.name().filter(|n| !n.is_empty() && n.as_str() != "Unknown").cloned())
        .collect();
    monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            let unique = os_names[i].as_ref().filter(|n| os_names.iter().filter(|o| o.as_ref() == Some(*n)).count() == 1);
            let screen = Screen {
                name: unique.cloned().unwrap_or_else(|| format!("Screen {}", i + 1)),
                width: m.size().width,
                height: m.size().height,
                primary: Some(*m.position()) == primary,
            };
            (screen, m)
        })
        .collect()
}

/// Which screen (its index in `screens`) `name` means: that one if connected, else the main
/// screen, else the first.
fn screen_index(app: &AppHandle, name: Option<&str>) -> Option<usize> {
    let list = screens(app);
    name.and_then(|n| list.iter().position(|(s, _)| s.name == n))
        .or_else(|| list.iter().position(|(s, _)| s.primary))
        .or(if list.is_empty() { None } else { Some(0) })
}

/// Whether the desktop session is Wayland, where apps can't move their own windows.
fn on_wayland() -> bool {
    cfg!(target_os = "linux")
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var("GDK_BACKEND").map_or(true, |b| !b.starts_with("x11"))
}

/// Puts the video window on `screen` (`None`: the main screen), fullscreen or not (decision
/// 0045). Returns false when a window that isn't fullscreen couldn't be moved: on Wayland apps
/// can't place their windows, so it has to be dragged there (or made fullscreen).
pub fn place(app: &AppHandle, screen: Option<&str>, fullscreen: bool) -> bool {
    let Some(window) = app.get_webview_window(LABEL) else { return true };
    let Some(index) = screen_index(app, screen) else { return true };
    if fullscreen {
        fullscreen_on(app, &window, index);
        return true;
    }
    leave_fullscreen(app, &window);
    if on_wayland() {
        return false;
    }
    if let Some((_, monitor)) = screens(app).into_iter().nth(index) {
        let p = monitor.position();
        let _ = window.set_position(PhysicalPosition::new(p.x + 40, p.y + 40));
    }
    true
}

// On Linux, fullscreen goes through GTK itself, in and out. Tauri keeps its own "is it
// fullscreen?" flag, set only when Tauri makes the window fullscreen, so after GTK's call it
// would still say no, and leaving fullscreen through Tauri would do nothing. GTK must be called
// from the main thread, so each call is queued there.

/// Fullscreen on screen number `index` (GDK's monitor number), through GTK: it asks the
/// compositor for that screen, the one way to choose a screen on Wayland (and it works on X11).
/// A window that's already fullscreen leaves fullscreen first and asks again a moment later:
/// some compositors keep a fullscreen window where it is when asked for another screen.
#[cfg(target_os = "linux")]
fn fullscreen_on(app: &AppHandle, window: &tauri::WebviewWindow, index: usize) {
    use gtk::prelude::{GtkWindowExt, WidgetExt};
    let name = screens(app).into_iter().nth(index).map(|(s, _)| s.name).unwrap_or_default();
    let window = window.clone();
    let _ = app.run_on_main_thread(move || {
        let gtk_window = window.gtk_window().ok();
        let screen = gtk_window.as_ref().and_then(GtkWindowExt::screen);
        let (Some(gtk_window), Some(screen)) = (gtk_window, screen) else {
            crate::logging::warn("Video window: GTK isn't available; fullscreen on the current screen");
            let _ = window.set_fullscreen(true);
            return;
        };
        let already = WidgetExt::window(&gtk_window)
            .is_some_and(|w| w.state().contains(gtk::gdk::WindowState::FULLSCREEN));
        crate::logging::info(&format!(
            "Video window: fullscreen on screen {} ({name}){}",
            index + 1,
            if already { ", leaving fullscreen first" } else { "" }
        ));
        if already {
            gtk_window.unfullscreen();
            gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                gtk_window.fullscreen_on_monitor(&screen, index as i32);
            });
        } else {
            gtk_window.fullscreen_on_monitor(&screen, index as i32);
        }
    });
}

/// Leaves fullscreen, through GTK (see above), whatever Tauri thinks.
#[cfg(target_os = "linux")]
fn leave_fullscreen(app: &AppHandle, window: &tauri::WebviewWindow) {
    use gtk::prelude::GtkWindowExt;
    let window = window.clone();
    let _ = app.run_on_main_thread(move || match window.gtk_window() {
        Ok(gtk_window) => gtk_window.unfullscreen(),
        Err(_) => {
            let _ = window.set_fullscreen(false);
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn leave_fullscreen(_app: &AppHandle, window: &tauri::WebviewWindow) {
    if window.is_fullscreen().unwrap_or(false) {
        let _ = window.set_fullscreen(false);
    }
}

/// Elsewhere: leave fullscreen, move onto the screen, fullscreen there.
#[cfg(not(target_os = "linux"))]
fn fullscreen_on(app: &AppHandle, window: &tauri::WebviewWindow, index: usize) {
    if window.is_fullscreen().unwrap_or(false) {
        let _ = window.set_fullscreen(false);
    }
    if let Some((_, monitor)) = screens(app).into_iter().nth(index) {
        let p = monitor.position();
        let _ = window.set_position(PhysicalPosition::new(p.x + 40, p.y + 40));
    }
    let _ = window.set_fullscreen(true);
}

/// Opens the video window (or brings it to the front), on the chosen screen and in fullscreen
/// if that's how it was left.
pub fn open(app: &AppHandle, state: &mut AppState) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_focus();
        return Ok(());
    }
    // No title bar: the window's own top bar is the drag handle. Files can't be dropped on it.
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Comilito video")
        .inner_size(960.0, 540.0)
        .min_inner_size(320.0, 180.0)
        .decorations(false)
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| format!("could not open the video window: {e}"))?;
    crate::video::allow_local_media(&window);
    drop(window);
    place(app, state.prefs.video.screen.as_deref(), state.prefs.video.fullscreen);
    state.video_window_open = true;
    Ok(())
}

pub fn close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.close();
    }
}

// ---- Commands -------------------------------------------------------------------------------
//
// The commands that can create the window are `async`, though nothing in them awaits. Tauri runs
// a sync command on the main thread, and on Windows WebView2 needs that same thread to finish
// building a window, so `build()` there waits on itself forever: a white window and a frozen app.
// An async command runs on a worker thread instead. (Tauri's docs for `build` warn about this.)
// The state lock is never held across an `.await` (there are none), which keeps the futures `Send`.

/// The header's video button: opens the window, or closes it if it's open.
#[tauri::command]
pub async fn toggle_video_window(app: AppHandle, state: AppStateRef<'_>) -> Result<(), String> {
    let mut s = lock(&state)?;
    if s.video_window_open {
        close(&app);
    } else {
        open(&app, &mut s)?;
    }
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// Opens the video window, or brings it to the front (the bottom bar's now-playing).
#[tauri::command]
pub async fn show_video_window(app: AppHandle, state: AppStateRef<'_>) -> Result<(), String> {
    let mut s = lock(&state)?;
    open(&app, &mut s)?;
    emit_state(&app, &s, Vec::new());
    Ok(())
}

#[tauri::command]
pub fn list_screens(app: AppHandle) -> Vec<Screen> {
    screens(&app).into_iter().map(|(s, _)| s).collect()
}

/// Moves the window to another screen and remembers it. Returns false when it couldn't be
/// moved (Wayland, not fullscreen): the window then says to drag it, or use fullscreen.
#[tauri::command]
pub fn set_video_screen(name: String, app: AppHandle, state: AppStateRef) -> Result<bool, String> {
    let mut s = lock(&state)?;
    let fullscreen = s.prefs.video.fullscreen;
    let moved = place(&app, Some(&name), fullscreen);
    s.prefs.video.screen = Some(name);
    s.save_prefs()?;
    let _ = app.emit("preferences-changed", &s.prefs);
    Ok(moved)
}

/// Fullscreen on or off (the top bar's button, or a double-click on it); remembered.
#[tauri::command]
pub fn set_video_fullscreen(on: bool, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    // Fullscreen goes to the chosen screen, not just the one the window happens to be on.
    place(&app, s.prefs.video.screen.as_deref(), on);
    s.prefs.video.fullscreen = on;
    s.save_prefs()?;
    let _ = app.emit("preferences-changed", &s.prefs);
    Ok(())
}

/// The video effects pad: plays a video effect over the scene's video until it ends, or stops
/// it if it's the one playing.
#[tauri::command]
pub async fn play_video_effect(id: String, app: AppHandle, state: AppStateRef<'_>) -> Result<(), String> {
    let mut s = lock(&state)?;
    if s.open_ref()?.project.video(&id).is_none() {
        return Err("that video no longer exists".into());
    }
    s.video_effect = if s.video_effect.as_deref() == Some(id.as_str()) { None } else { Some(id) };
    // Video effects play in the window, so it opens if it wasn't.
    if s.video_effect.is_some() && !s.video_window_open {
        open(&app, &mut s)?;
    }
    emit_state(&app, &s, Vec::new());
    Ok(())
}

/// The window says a video effect has finished; the playlist comes back.
#[tauri::command]
pub fn video_effect_ended(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    if s.video_effect.as_deref() == Some(id.as_str()) {
        s.video_effect = None;
        emit_state(&app, &s, Vec::new());
    }
    Ok(())
}

// ---- Where each video left off, across restarts ---------------------------------------------
//
// Kept in `video-positions.json` in the app data folder, per project folder, not in the
// project: positions change every few seconds while videos play, and a synced project folder
// would upload comilito.json each time. Written when the project closes (quitting included)
// and at most every 30 seconds meanwhile.

const POSITIONS_FILE: &str = "video-positions.json";

type AllPositions = HashMap<String, HashMap<String, u64>>;

fn read_all_positions(data_dir: &Path) -> AllPositions {
    std::fs::read_to_string(data_dir.join(POSITIONS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Where the videos of the project in `project_dir` left off (ms), from the last time.
pub fn load_positions(data_dir: &Path, project_dir: &Path) -> HashMap<String, u64> {
    read_all_positions(data_dir).remove(&project_dir.to_string_lossy().to_string()).unwrap_or_default()
}

/// Saves `positions` for the project in `project_dir`, keeping only videos it still has.
pub fn store_positions(data_dir: &Path, project_dir: &Path, project: &crate::project::Project, positions: &HashMap<String, u64>) {
    let mut all = read_all_positions(data_dir);
    let kept: HashMap<String, u64> = positions
        .iter()
        .filter(|(id, _)| project.video(id).is_some())
        .map(|(id, ms)| (id.clone(), *ms))
        .collect();
    let key = project_dir.to_string_lossy().to_string();
    if kept.is_empty() {
        all.remove(&key);
    } else {
        all.insert(key, kept);
    }
    let result = serde_json::to_vec_pretty(&all)
        .map_err(|e| e.to_string())
        .and_then(|json| crate::storage::write_atomically(&data_dir.join(POSITIONS_FILE), &json));
    if let Err(e) = result {
        crate::logging::warn(&format!("could not remember where the videos left off: {e}"));
    }
}

/// Remembers where a video stopped (ms), so it can resume there, also after a restart.
#[tauri::command]
pub fn save_video_position(id: String, position_ms: u64, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.video_positions.insert(id, position_ms);
    // Also to disk now and then, so a crash loses at most half a minute.
    if s.positions_saved.elapsed() >= std::time::Duration::from_secs(30)
        && let Some(open) = &s.open
    {
        store_positions(&s.data_dir, &open.dir, &open.project, &s.video_positions);
        s.positions_saved = std::time::Instant::now();
    }
    Ok(())
}

#[tauri::command]
pub fn video_positions(state: AppStateRef) -> Result<HashMap<String, u64>, String> {
    Ok(lock(&state)?.video_positions.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::ProjectDefaults, storage::test_support::TempDir};

    #[test]
    fn positions_are_kept_per_project_and_only_for_existing_videos() {
        let data = TempDir::new();
        let mut project = crate::project::Project::new("P", ProjectDefaults::default());
        let file = crate::project::FileInfo {
            path: "video/ambience/a.webm".into(),
            size: 1,
            modified: String::new(),
            sha256: None,
            loudness: None,
        };
        let video = crate::video::new_video("a".into(), file, crate::project::VideoKind::Ambience);
        let id = video.id.clone();
        project.videos.push(video);

        let positions = HashMap::from([(id.clone(), 42_000), ("vid-gone".to_string(), 7)]);
        store_positions(&data.0, Path::new("/projects/Strahd"), &project, &positions);
        store_positions(&data.0, Path::new("/projects/Other"), &project, &HashMap::from([(id.clone(), 1)]));

        let back = load_positions(&data.0, Path::new("/projects/Strahd"));
        assert_eq!(back, HashMap::from([(id.clone(), 42_000)]), "the removed video's position is dropped");
        assert_eq!(load_positions(&data.0, Path::new("/projects/Other"))[&id], 1);
        assert!(load_positions(&data.0, Path::new("/projects/New")).is_empty());
    }
}
