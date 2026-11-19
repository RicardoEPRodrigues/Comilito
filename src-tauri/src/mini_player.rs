//! The mini player (decision 0049): the main window shrinks to a small layout that stays on top
//! of other windows, and grows back to where it was.
//!
//! The window's normal size and place are saved to `.mini-player.json` (next to
//! `preferences.json`) while the mini player is on, and the file is removed when it goes off.
//! Why a file and not just memory: the window-state plugin remembers whatever size the window
//! has when the app quits. Quitting in the mini player therefore saves the mini size as "the
//! window size", and the app starts small next time. With the file still there, the app knows
//! it was left in the mini player, starts in it again, and can still grow back to the size
//! from before.
//!
//! The frontend decides the layout; these commands only move and resize the window. The mini
//! player also follows the window's width: made narrow by hand it turns on, widened it turns off
//! (`resize: false`: the user already sized the window, so it's left as it is).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, WebviewWindow};

use crate::commands::{AppStateRef, lock};
use crate::storage;

const FILE: &str = ".mini-player.json";

/// The mini player's size, in logical pixels (CSS pixels at 100%).
const MINI_WIDTH: f64 = 380.0;
const MINI_HEIGHT: f64 = 640.0;
/// Going back to the full window when there's no size to go back to (the window was made
/// narrow by hand): tauri.conf.json's first size.
const FULL_WIDTH: f64 = 1280.0;
const FULL_HEIGHT: f64 = 800.0;

/// The normal window, in physical pixels (what the OS reports and accepts back unchanged).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Placement {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

/// What `.mini-player.json` holds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Saved {
    /// Where the window goes back to. `None` if the window was made narrow by hand, or its
    /// placement couldn't be read; going back then gives it the default size.
    normal: Option<Placement>,
    on_top: bool,
}

/// What the frontend needs to know.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MiniPlayer {
    pub on: bool,
    pub on_top: bool,
}

fn file_in(config_dir: &Path) -> PathBuf {
    config_dir.join(FILE)
}

/// `None` when the mini player is off (no file, or one that can't be read).
fn read(config_dir: &Path) -> Option<Saved> {
    let text = std::fs::read_to_string(file_in(config_dir)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write(config_dir: &Path, saved: &Saved) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("could not create {}: {e}", config_dir.display()))?;
    let json = serde_json::to_string_pretty(saved).map_err(|e| e.to_string())?;
    storage::write_atomically(&file_in(config_dir), json.as_bytes())
}

fn remove(config_dir: &Path) {
    // Missing already is fine: the mini player is off either way.
    let _ = std::fs::remove_file(file_in(config_dir));
}

/// Where the window is now. `?` inside works on `tauri::Result`; any error gives `None`.
fn placement(window: &WebviewWindow) -> Option<Placement> {
    let position = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    Some(Placement {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized: window.is_maximized().unwrap_or(false),
    })
}

/// Called once by the frontend when it starts: is the mini player on (left on when the app
/// last quit)? If so, the window is put back on top, which the OS doesn't remember.
#[tauri::command]
pub fn mini_player(window: WebviewWindow, state: AppStateRef) -> Result<MiniPlayer, String> {
    let config_dir = lock(&state)?.config_dir.clone();
    let Some(saved) = read(&config_dir) else {
        return Ok(MiniPlayer { on: false, on_top: false });
    };
    if saved.on_top {
        window.set_always_on_top(true).map_err(|e| e.to_string())?;
    }
    Ok(MiniPlayer { on: true, on_top: saved.on_top })
}

/// Turns the mini player on or off.
///
/// With `resize` (the header's button, the back button): on shrinks the window and keeps it on
/// top; off puts it back to the size and place from before. Without it (the window was resized
/// by hand across the narrow width), only the mini player's state changes: the window keeps its
/// size and doesn't go on top. Turning it on while it's on changes nothing.
#[tauri::command]
pub fn set_mini_player(on: bool, resize: bool, window: WebviewWindow, state: AppStateRef) -> Result<MiniPlayer, String> {
    let config_dir = lock(&state)?.config_dir.clone();
    let err = |e: tauri::Error| e.to_string();
    match (on, read(&config_dir)) {
        (true, Some(saved)) => Ok(MiniPlayer { on: true, on_top: saved.on_top }),
        (true, None) if !resize => {
            write(&config_dir, &Saved { normal: None, on_top: false })?;
            Ok(MiniPlayer { on: true, on_top: false })
        }
        (true, None) => {
            // Saved before touching the window, so a failure below still leaves a way back.
            write(&config_dir, &Saved { normal: placement(&window), on_top: true })?;
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize().map_err(err)?;
            }
            window.set_size(LogicalSize::new(MINI_WIDTH, MINI_HEIGHT)).map_err(err)?;
            window.set_always_on_top(true).map_err(err)?;
            Ok(MiniPlayer { on: true, on_top: true })
        }
        (false, saved) => {
            window.set_always_on_top(false).map_err(err)?;
            if resize {
                match saved.and_then(|s| s.normal) {
                    Some(normal) => {
                        window.set_size(PhysicalSize::new(normal.width, normal.height)).map_err(err)?;
                        // Wayland doesn't let apps place their windows; there the window stays
                        // where it is and only its size changes. Not an error worth reporting.
                        let _ = window.set_position(PhysicalPosition::new(normal.x, normal.y));
                        if normal.maximized {
                            window.maximize().map_err(err)?;
                        }
                    }
                    None => window.set_size(LogicalSize::new(FULL_WIDTH, FULL_HEIGHT)).map_err(err)?,
                }
            }
            remove(&config_dir);
            Ok(MiniPlayer { on: false, on_top: false })
        }
    }
}

/// The mini player's pin: keep the window on top of others, or not.
#[tauri::command]
pub fn set_mini_on_top(on_top: bool, window: WebviewWindow, state: AppStateRef) -> Result<MiniPlayer, String> {
    let config_dir = lock(&state)?.config_dir.clone();
    let Some(mut saved) = read(&config_dir) else {
        return Err("the mini player isn't on".into());
    };
    window.set_always_on_top(on_top).map_err(|e| e.to_string())?;
    saved.on_top = on_top;
    write(&config_dir, &saved)?;
    Ok(MiniPlayer { on: true, on_top })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    #[test]
    fn saved_placement_round_trips_and_removing_turns_it_off() {
        let dir = TempDir::new();
        assert_eq!(read(&dir.0), None, "off when there's no file");

        let saved = Saved {
            normal: Some(Placement { x: -1200, y: 40, width: 2560, height: 1440, maximized: true }),
            on_top: false,
        };
        write(&dir.0, &saved).unwrap();
        assert_eq!(read(&dir.0), Some(saved));

        remove(&dir.0);
        assert_eq!(read(&dir.0), None);
    }

    #[test]
    fn unreadable_file_counts_as_off() {
        let dir = TempDir::new();
        std::fs::write(file_in(&dir.0), "not json").unwrap();
        assert_eq!(read(&dir.0), None);
    }
}
