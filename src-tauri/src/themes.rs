//! User themes (decision 0016): JSON files in `<config>/themes/`, one per theme.
//!
//! The built-in themes (Codex, Ironwood, Hearth) live in the frontend (`src/lib/themes.ts`)
//! and are read-only. Editing one makes a user copy, stored here. Rust doesn't interpret the
//! colours; it only checks that each is a `#rgb`/`#rrggbb` value, so a hand-edited or
//! imported file can't inject anything else into the page's CSS.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::storage;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    /// User themes have ids starting with `user-`.
    pub id: String,
    pub name: String,
    /// Whether it's a dark theme (used by "follow the desktop" light/dark pairing).
    pub dark: bool,
    /// Token name → colour, e.g. "bg" → "#f4f2e9". A `BTreeMap` keeps the keys sorted, so
    /// saved files are stable and easy to diff.
    pub colors: BTreeMap<String, String>,
    pub fonts: ThemeFonts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFonts {
    pub display: String,
    pub body: String,
}

fn themes_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("themes")
}

fn path_for(config_dir: &Path, id: &str) -> PathBuf {
    themes_dir(config_dir).join(format!("{}.json", storage::safe_file_name(id)))
}

/// `#rgb` or `#rrggbb`.
fn is_hex_colour(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Font names may only hold letters, digits, spaces and a few safe characters.
fn is_safe_font(name: &str) -> bool {
    !name.is_empty() && name.len() <= 60 && name.chars().all(|c| c.is_alphanumeric() || " -_".contains(c))
}

fn validate(theme: &Theme) -> Result<(), String> {
    if theme.name.trim().is_empty() {
        return Err("the theme needs a name".into());
    }
    if let Some((key, value)) = theme.colors.iter().find(|(_, v)| !is_hex_colour(v)) {
        return Err(format!("\"{key}\" is not a colour: {value}"));
    }
    if !is_safe_font(&theme.fonts.display) || !is_safe_font(&theme.fonts.body) {
        return Err("a font name has unsupported characters".into());
    }
    Ok(())
}

/// Every valid user theme, sorted by name. Unreadable files are skipped.
pub fn list(config_dir: &Path) -> Vec<Theme> {
    let Ok(entries) = fs::read_dir(themes_dir(config_dir)) else {
        return Vec::new();
    };
    let mut themes: Vec<Theme> = entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|text| serde_json::from_str::<Theme>(&text).ok())
        .filter(|t| t.id.starts_with("user-") && validate(t).is_ok())
        .collect();
    themes.sort_by_key(|t| t.name.to_lowercase());
    themes
}

pub fn save(config_dir: &Path, theme: &Theme) -> Result<(), String> {
    if !theme.id.starts_with("user-") {
        return Err("built-in themes can't be changed; duplicate it first".into());
    }
    validate(theme)?;
    fs::create_dir_all(themes_dir(config_dir)).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(theme).map_err(|e| e.to_string())?;
    storage::write_atomically(&path_for(config_dir, &theme.id), json.as_bytes())
}

pub fn delete(config_dir: &Path, id: &str) -> Result<(), String> {
    let path = path_for(config_dir, id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("could not delete the theme: {e}"))?;
    }
    Ok(())
}

/// Reads a theme file someone shared. It always gets a fresh id, so importing never
/// overwrites an existing theme.
pub fn import(config_dir: &Path, file: &Path) -> Result<Theme, String> {
    let text = fs::read_to_string(file).map_err(|e| format!("could not read {}: {e}", file.display()))?;
    let mut theme: Theme =
        serde_json::from_str(&text).map_err(|_| "that file isn't a Comilito theme".to_string())?;
    theme.id = format!("user-{}", uuid::Uuid::new_v4().simple());
    save(config_dir, &theme)?;
    Ok(theme)
}

pub fn export(theme: &Theme, dest: &Path) -> Result<(), String> {
    validate(theme)?;
    let json = serde_json::to_string_pretty(theme).map_err(|e| e.to_string())?;
    fs::write(dest, json).map_err(|e| format!("could not write {}: {e}", dest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    fn theme(id: &str) -> Theme {
        Theme {
            id: id.into(),
            name: "My Tavern".into(),
            dark: false,
            colors: BTreeMap::from([("bg".into(), "#efe6d2".into()), ("accent".into(), "#b5852f".into())]),
            fonts: ThemeFonts {
                display: "MedievalSharp".into(),
                body: "Inter".into(),
            },
        }
    }

    #[test]
    fn saves_lists_imports_and_deletes() {
        let dir = TempDir::new();
        save(&dir.0, &theme("user-1")).unwrap();
        assert_eq!(list(&dir.0).len(), 1);

        let file = dir.0.join("shared.json");
        export(&theme("user-1"), &file).unwrap();
        let imported = import(&dir.0, &file).unwrap();
        assert_ne!(imported.id, "user-1", "imports get a new id");
        assert_eq!(list(&dir.0).len(), 2);

        delete(&dir.0, "user-1").unwrap();
        assert_eq!(list(&dir.0).len(), 1);
    }

    #[test]
    fn rejects_built_in_ids_and_unsafe_values() {
        let dir = TempDir::new();
        assert!(save(&dir.0, &theme("codex")).is_err());
        let mut bad = theme("user-2");
        bad.colors.insert("bg".into(), "red; background: url(x)".into());
        assert!(save(&dir.0, &bad).is_err());
        let mut bad_font = theme("user-3");
        bad_font.fonts.display = "x'); }".into();
        assert!(save(&dir.0, &bad_font).is_err());
    }
}
