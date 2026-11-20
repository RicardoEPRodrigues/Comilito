//! Moving files and folders to the desktop's trash, also from inside the Flatpak.
//!
//! The `trash` crate does it by hand, as the freedesktop spec describes: it moves the file into
//! `$XDG_DATA_HOME/Trash`. Inside the Flatpak `$XDG_DATA_HOME` is the sandbox's own folder
//! (`~/.var/app/<id>/data`), so files "trashed" that way would land in a trash no file manager
//! shows, and could never be restored. There the trash portal does it instead: Comilito hands
//! it the open file, and the portal, which runs outside the sandbox, moves it to the real trash.
//! (Named `trash_bin` so it doesn't clash with the `trash` crate.)

use std::path::Path;

/// Moves `path` (a file or a folder) to the trash.
pub fn delete(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::flatpak::in_flatpak() {
        return portal::trash(path);
    }
    trash::delete(path).map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
mod portal {
    use std::{os::fd::AsFd, os::unix::fs::OpenOptionsExt, path::Path};

    use zbus::zvariant::Fd;

    /// `org.freedesktop.portal.Trash.TrashFile`. It takes a file descriptor rather than a path,
    /// so the portal acts on exactly the file the app could open. `O_PATH` opens a file or a
    /// folder without reading it: enough to name it, and it works whatever the permissions.
    pub fn trash(path: &Path) -> Result<(), String> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
            .open(path)
            .map_err(|e| format!("could not open {}: {e}", path.display()))?;
        let conn = zbus::blocking::Connection::session().map_err(|e| format!("no session bus: {e}"))?;
        let reply = conn
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.portal.Trash"),
                "TrashFile",
                // A one-element tuple: D-Bus arguments always go as a tuple, hence the comma.
                &(Fd::from(file.as_fd()),),
            )
            .map_err(|e| format!("the trash portal failed: {e}"))?;
        // 1 means it worked; 0 that it didn't (the portal gives no reason).
        match reply.body().deserialize::<u32>() {
            Ok(1) => Ok(()),
            _ => Err("the desktop refused to move it to the trash".into()),
        }
    }
}
