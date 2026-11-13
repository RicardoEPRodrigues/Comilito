mod audio;
mod project;

use std::sync::Mutex;

use audio::{AudioEngine, Volumes};
use project::Category;
use tauri::State;

/// Returns the app version from Cargo.toml.
///
/// `#[tauri::command]` makes this callable from the frontend with `invoke("app_version")`.
/// `env!` reads the value at compile time, so there's no runtime lookup.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Temporary: plays an audio file by absolute path, to test the engine before the
/// project model exists. Will be replaced by `play_sound(id)`.
///
/// `State<Mutex<AudioEngine>>` asks Tauri for the engine registered with `.manage()` below.
/// Tauri may run commands on several threads at once, so the engine sits behind a `Mutex`.
/// Unlike C++'s std::mutex, a Rust Mutex *contains* the data: the only way to reach the
/// engine is through `lock()`, so forgetting to lock is a compile error, not a race.
#[tauri::command]
fn play_file(
    path: String,
    category: Category,
    engine: State<Mutex<AudioEngine>>,
) -> Result<(), String> {
    // `lock()` fails only if another thread panicked while holding the lock.
    // The guard it returns unlocks automatically when it goes out of scope (like C++ RAII).
    let mut engine = engine.lock().map_err(|_| "audio engine lock poisoned")?;
    engine.play_file(&path, category)
}

/// Current master and category volumes, so the UI can show them.
#[tauri::command]
fn get_volumes(engine: State<Mutex<AudioEngine>>) -> Result<Volumes, String> {
    let engine = engine.lock().map_err(|_| "audio engine lock poisoned")?;
    Ok(engine.volumes())
}

/// Sets the master volume (0.0–1.0).
#[tauri::command]
fn set_master_volume(volume: f32, engine: State<Mutex<AudioEngine>>) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|_| "audio engine lock poisoned")?;
    engine.set_master_volume(volume);
    Ok(())
}

/// Sets one category's volume (0.0–1.0). `category` arrives from JS as "music",
/// "ambience" or "effects"; Tauri rejects anything else before this function runs.
#[tauri::command]
fn set_category_volume(
    category: Category,
    volume: f32,
    engine: State<Mutex<AudioEngine>>,
) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|_| "audio engine lock poisoned")?;
    engine.set_category_volume(category, volume);
    Ok(())
}

/// Fades out and stops everything that's playing.
#[tauri::command]
fn stop_all(engine: State<Mutex<AudioEngine>>) -> Result<(), String> {
    let mut engine = engine.lock().map_err(|_| "audio engine lock poisoned")?;
    engine.stop_all();
    Ok(())
}

// On Android this function becomes the app's entry point; on desktop main.rs calls it.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Without audio output the app is useless, so fail loudly at startup.
    // `expect` stops the program with this message if `new()` returned an error.
    // TODO: show this in the UI instead once there's somewhere to show it.
    let engine = AudioEngine::new().expect("failed to start audio engine");

    tauri::Builder::default()
        // Hands ownership of the engine to Tauri, which shares it with every command.
        .manage(Mutex::new(engine))
        .invoke_handler(tauri::generate_handler![
            app_version,
            play_file,
            stop_all,
            get_volumes,
            set_master_volume,
            set_category_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
