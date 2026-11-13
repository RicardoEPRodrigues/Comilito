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
