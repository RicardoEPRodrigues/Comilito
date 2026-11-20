//! What's different when Comilito runs inside the Flatpak's sandbox (decision 0059).

use std::path::Path;

/// Flatpak puts `/.flatpak-info` at the root of every sandbox; it's how apps are meant to tell.
pub fn in_flatpak() -> bool {
    cfg!(target_os = "linux") && Path::new("/.flatpak-info").is_file()
}

/// Turns off WebKit's DMA-BUF renderer inside the Flatpak on NVIDIA GPUs. With the GNOME
/// runtime's WebKitGTK and NVIDIA's driver it can't allocate its buffers ("Failed to create GBM
/// buffer … Invalid argument") and the window stays white. The older renderer it falls back to
/// works. Intel and AMD keep the DMA-BUF renderer, and so does Comilito outside the Flatpak,
/// where the system's WebKitGTK works on NVIDIA. A value the user set wins.
#[cfg(target_os = "linux")]
pub fn webkit_renderer_workaround() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    // The sandbox shows NVIDIA's device nodes (`--device=dri`) and the driver's /proc entry.
    let nvidia = Path::new("/proc/driver/nvidia/version").exists() || Path::new("/dev/nvidiactl").exists();
    if !in_flatpak() || !nvidia || std::env::var_os(VAR).is_some() {
        return;
    }
    // `set_var` is `unsafe` since Rust 2024 (see `private_gstreamer_registry` in lib.rs). This
    // runs first thing in `run`, before any other thread exists, and WebKit's processes
    // inherit it when they start.
    unsafe { std::env::set_var(VAR, "1") };
}

/// Refuses folders the sandbox can't really write to, before a project is opened or created
/// there. The Flatpak reaches the home folder and drives under /media, /run/media and /mnt.
/// A folder anywhere else, picked in the file chooser, comes back as a read-only copy under
/// `/run/user/<uid>/doc/` (the document portal): opening a project there would fail on every save.
pub fn check_writable_folder(path: &Path) -> Result<(), String> {
    if in_flatpak() && is_document_portal_path(path) {
        return Err(format!(
            "Comilito can't save in “{}”: the desktop lent it this folder read-only. Choose it again \
             with More › Open a project folder…. If that doesn't help, the folder is somewhere \
             Comilito can't write to: it can write to your home folder and to drives under /media, \
             /run/media or /mnt. Move the folder to one of those places, or give Comilito access \
             to it (for example with Flatseal).",
            path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()
        ));
    }
    Ok(())
}

/// `/run/user/<uid>/doc/…`, where the document portal shows files it lends to sandboxed apps.
fn is_document_portal_path(path: &Path) -> bool {
    let mut parts = path.components().map(|c| c.as_os_str().to_string_lossy());
    // `components` starts with the root ("/"), then "run", "user", the uid, "doc".
    matches!(
        (parts.next().as_deref(), parts.next().as_deref(), parts.next().as_deref(), parts.next(), parts.next().as_deref()),
        (Some("/"), Some("run"), Some("user"), Some(_), Some("doc"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_document_portal_paths() {
        assert!(is_document_portal_path(Path::new("/run/user/1000/doc/e12efabf/Into the Barrens")));
        assert!(!is_document_portal_path(Path::new("/run/user/1000/gvfs/x")));
        assert!(!is_document_portal_path(Path::new("/home/someone/Documents/Comilito/x")));
        assert!(!is_document_portal_path(Path::new("/media/someone/data/x")));
    }
}
