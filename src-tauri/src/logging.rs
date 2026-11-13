//! A plain-text log file, for "See logs" (TODO.md) and for reporting problems.
//!
//! Every notice shown to the user, every error from the UI and the important background
//! events (downloads, tool installs, backups) are appended to `<app data>/logs/comilito.log`
//! with a timestamp. When the file passes 1 MB it's renamed to `comilito.old.log` and a new
//! one starts, so it never grows without bound.
//!
//! The log is a global (a `static`), like a C++ singleton: any module can call
//! `logging::info(...)` without passing a handle around. `Mutex` makes it safe to write from
//! several threads; `OnceLock` lets it be set once at startup.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

const MAX_BYTES: u64 = 1024 * 1024;

static LOG: OnceLock<Mutex<Logger>> = OnceLock::new();

struct Logger {
    path: PathBuf,
    file: Option<File>,
}

/// Starts logging into `dir/comilito.log`. Called once at startup.
pub fn init(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    let path = dir.join("comilito.log");
    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    let _ = LOG.set(Mutex::new(Logger { path, file }));
    info(&format!("Comilito {} started", env!("CARGO_PKG_VERSION")));
}

/// Writes panics (Rust's crashes) to the log before the usual message on stderr, so a crash
/// leaves a trace even when nobody is watching a terminal.
pub fn log_panics() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        error(&format!("crashed: {info}"));
        previous(info);
    }));
}

/// When Comilito is started without a terminal (double-clicking an AppImage, a desktop
/// launcher), whatever it or its libraries print on stderr is lost: GTK and WebKit errors,
/// a panic's message. This sends stderr (file descriptor 2) to `<app data>/logs/stderr.log`
/// instead, so those messages can be read afterwards.
///
/// It runs first thing in `run()`, before Tauri starts, so that WebKit's helper processes
/// (which inherit stderr when they start) are covered too. Tauri's app-data folder isn't
/// known yet then, so it's worked out the same way Tauri does on Linux:
/// `$XDG_DATA_HOME/<identifier>`, which defaults to `~/.local/share/<identifier>`.
#[cfg(target_os = "linux")]
pub fn capture_stderr(identifier: &str) {
    use std::io::IsTerminal;
    use std::os::fd::AsRawFd;

    if std::io::stderr().is_terminal() {
        return; // Someone is watching: leave it alone.
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let Some(data) = data else { return };
    let dir = data.join(identifier).join("logs");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("stderr.log");
    // Start over when it gets big; it's only for the latest problems.
    if path.metadata().map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
        let _ = fs::remove_file(&path);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else { return };
    let _ = writeln!(file, "--- {} Comilito {} started without a terminal", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"), env!("CARGO_PKG_VERSION"));
    // `dup2` makes file descriptor 2 point at our file (C's dup2; Rust's std has no wrapper).
    // `unsafe` because it's a raw C call: Rust can't check it, we promise the fds are valid.
    unsafe {
        libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO);
    }
    // fd 2 now refers to the file by itself, so this handle can be closed normally.
}

/// Where the log file is, for "See logs".
pub fn path() -> Option<PathBuf> {
    LOG.get().and_then(|l| l.lock().ok()).map(|l| l.path.clone())
}

pub fn info(message: &str) {
    write("INFO", message);
}

pub fn warn(message: &str) {
    write("WARN", message);
}

pub fn error(message: &str) {
    write("ERROR", message);
}

fn write(level: &str, message: &str) {
    // Also print to the terminal, which is where developers look first.
    eprintln!("[{level}] {message}");
    let Some(lock) = LOG.get() else { return };
    let Ok(mut logger) = lock.lock() else { return };
    let size = logger.path.metadata().map(|m| m.len()).unwrap_or(0);
    if size > MAX_BYTES {
        let old = logger.path.with_file_name("comilito.old.log");
        logger.file = None; // close before renaming (needed on Windows)
        let _ = fs::rename(&logger.path, old);
        logger.file = OpenOptions::new().create(true).append(true).open(&logger.path).ok();
    }
    let time = crate::storage::format_time(std::time::SystemTime::now());
    if let Some(file) = logger.file.as_mut() {
        let _ = writeln!(file, "{time} {level:5} {message}");
    }
}
