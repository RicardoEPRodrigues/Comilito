//! Reads the desktop's accent colour and light/dark preference (decision 0016), so the UI
//! can follow them. Each OS keeps these somewhere different:
//!
//! - Linux: the XDG desktop portal (`org.freedesktop.appearance` in the Settings interface),
//!   which GNOME 47+, KDE Plasma 6 and COSMIC implement. Read over D-Bus with `zbus`.
//! - Windows: the `AccentColor` value in the registry, read with the built-in `reg` tool.
//! - macOS: `AppleAccentColor` in the global defaults, read with the `defaults` tool.
//!
//! Anything that can't be read is `None`, and the UI falls back to the theme's own colour.
//! A background thread (see `watch`) polls every couple of seconds and tells the UI when
//! something changed, so changing the accent in the desktop's settings updates the app live.

use std::{thread, time::Duration};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopAppearance {
    /// `#rrggbb`, if the desktop has an accent colour.
    pub accent: Option<String>,
    /// Whether the desktop prefers a dark look, if it says.
    pub dark: Option<bool>,
}

/// Reads the current desktop appearance. Never fails: unknown parts are `None`.
pub fn read() -> DesktopAppearance {
    platform::read()
}

/// Polls the desktop appearance and emits `desktop-appearance` to the UI whenever it changes.
pub fn watch(app: AppHandle) {
    let mut last = read();
    loop {
        thread::sleep(Duration::from_secs(2));
        let now = read();
        if now != last {
            let _ = app.emit("desktop-appearance", &now);
            last = now;
        }
    }
}

/// `#rrggbb` from red/green/blue channels in 0.0–1.0.
fn hex_from_unit(r: f64, g: f64, b: f64) -> Option<String> {
    let in_range = |c: f64| (0.0..=1.0).contains(&c);
    if !(in_range(r) && in_range(g) && in_range(b)) {
        // The portal uses values outside 0–1 to mean "no accent set".
        return None;
    }
    let to_byte = |c: f64| (c * 255.0).round() as u8;
    Some(format!("#{:02x}{:02x}{:02x}", to_byte(r), to_byte(g), to_byte(b)))
}

// `#[cfg(...)]` compiles a block only for some targets, like `#ifdef` in C. Each OS gets its
// own `platform` module with the same `read()` function.

#[cfg(target_os = "linux")]
mod platform {
    use zbus::zvariant::{OwnedValue, Structure, Value};

    use super::{DesktopAppearance, hex_from_unit};

    pub fn read() -> DesktopAppearance {
        let Ok(conn) = zbus::blocking::Connection::session() else {
            return DesktopAppearance::default();
        };
        let accent = read_setting(&conn, "accent-color").and_then(|v| accent_from(&v));
        // 0 = no preference, 1 = prefer dark, 2 = prefer light.
        let dark = read_setting(&conn, "color-scheme").and_then(|v| match u32::try_from(&v) {
            Ok(1) => Some(true),
            Ok(2) => Some(false),
            _ => None,
        });
        DesktopAppearance { accent, dark }
    }

    /// Reads one `org.freedesktop.appearance` setting from the portal. Newer portals have
    /// `ReadOne`; older ones only `Read`, which wraps the value in one more variant.
    fn read_setting(conn: &zbus::blocking::Connection, key: &str) -> Option<OwnedValue> {
        let call = |method: &str| {
            conn.call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.portal.Settings"),
                method,
                &("org.freedesktop.appearance", key),
            )
            .ok()
            .and_then(|reply| reply.body().deserialize::<OwnedValue>().ok())
        };
        let value = call("ReadOne").or_else(|| call("Read"))?;
        // Unwrap nested variants (`Read` returns `v` inside `v`).
        let mut inner: Value = value.into();
        while let Value::Value(boxed) = inner {
            inner = *boxed;
        }
        inner.try_into_owned().ok()
    }

    /// The accent is a structure of three doubles: (red, green, blue), each 0.0–1.0.
    pub(super) fn accent_from(value: &OwnedValue) -> Option<String> {
        let structure: &Structure = value.downcast_ref().ok()?;
        let channels: Vec<f64> = structure
            .fields()
            .iter()
            .filter_map(|f| f64::try_from(f).ok())
            .collect();
        match channels[..] {
            [r, g, b] => hex_from_unit(r, g, b),
            _ => None,
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::process::Command;

    use super::DesktopAppearance;

    pub fn read() -> DesktopAppearance {
        // AccentColor is a DWORD in ABGR order: 0xAABBGGRR.
        let accent = reg_dword(r"HKCU\Software\Microsoft\Windows\DWM", "AccentColor").map(|v| {
            let r = v & 0xff;
            let g = (v >> 8) & 0xff;
            let b = (v >> 16) & 0xff;
            format!("#{r:02x}{g:02x}{b:02x}")
        });
        // AppsUseLightTheme: 0 = dark, 1 = light.
        let dark = reg_dword(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "AppsUseLightTheme",
        )
        .map(|v| v == 0);
        DesktopAppearance { accent, dark }
    }

    /// Runs `reg query <key> /v <name>` and parses the `0x...` number it prints.
    fn reg_dword(key: &str, name: &str) -> Option<u32> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000; // don't flash a console window
        let out = Command::new("reg")
            .args(["query", key, "/v", name])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let hex = text.split_whitespace().find(|w| w.starts_with("0x"))?;
        u32::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    use super::DesktopAppearance;

    pub fn read() -> DesktopAppearance {
        let defaults = |key: &str| {
            Command::new("defaults")
                .args(["read", "-g", key])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        };
        // AppleAccentColor is an index into the system palette; missing means blue.
        let accent = Some(
            match defaults("AppleAccentColor").as_deref() {
                Some("-1") => "#8c8c8c", // graphite
                Some("0") => "#ff5257",  // red
                Some("1") => "#f7821b",  // orange
                Some("2") => "#ffc600",  // yellow
                Some("3") => "#62ba46",  // green
                Some("5") => "#a550a7",  // purple
                Some("6") => "#f74f9e",  // pink
                _ => "#007aff",          // blue (default)
            }
            .to_string(),
        );
        let dark = Some(defaults("AppleInterfaceStyle").as_deref() == Some("Dark"));
        DesktopAppearance { accent, dark }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod platform {
    use super::DesktopAppearance;

    pub fn read() -> DesktopAppearance {
        DesktopAppearance::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_unit_channels_to_hex() {
        assert_eq!(hex_from_unit(1.0, 0.5, 0.0).as_deref(), Some("#ff8000"));
        assert_eq!(hex_from_unit(-1.0, 0.0, 0.0), None, "out of range means no accent");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn decodes_the_portal_accent_structure() {
        use zbus::zvariant::{OwnedValue, Value};
        // The portal sends the accent as a structure of three doubles.
        let value: OwnedValue = Value::from((0.2f64, 0.4f64, 0.6f64)).try_into_owned().unwrap();
        assert_eq!(platform::accent_from(&value).as_deref(), Some("#336699"));
    }

    #[test]
    fn reading_never_panics() {
        // Whatever the machine running the tests has (or hasn't), this must not fail.
        let _ = read();
    }
}

