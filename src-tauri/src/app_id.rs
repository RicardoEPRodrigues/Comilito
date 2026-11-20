//! The app's identifier (decision 0059).
//!
//! Tauri names the app's own folders after `identifier` in tauri.conf.json: on Linux
//! `~/.local/share/<id>` (logs, backups, downloaded tools, WebKit's storage), `~/.config/<id>`
//! and `~/.cache/<id>`. It's also the Flatpak app ID, which desktops use to match a window to
//! its launcher and icon.

/// Must equal `identifier` in tauri.conf.json (a test checks). Code that runs before Tauri
/// starts, or outside it, needs the name and can't ask Tauri for it.
pub const APP_ID: &str = "me.colorcrow.comilito";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_identifier_in_tauri_conf() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], APP_ID);
    }
}
