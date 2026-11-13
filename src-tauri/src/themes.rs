//! User themes (decisions 0016 and 0025): JSON files in `<config>/themes/`, one per theme,
//! plus the files themes bring with them (a watermark, fonts, licences) in
//! `<config>/themes/assets/`.
//!
//! The built-in themes (Codex, Ironwood, Hearth) live in the frontend (`src/lib/themes.ts`)
//! and are read-only. Editing one makes a user copy, stored here. Rust doesn't interpret the
//! colours; it only checks that each is a `#rgb`/`#rrggbb` value, so a hand-edited or
//! imported file can't inject anything else into the page's CSS.
//!
//! Bundled files are stored under their SHA-256 hash (`<hash>.png`). That gives every file
//! a name that can't point anywhere else on disk, lets two themes (a theme and its copy)
//! share one file, and makes "is this file still used?" a simple question when one is deleted.
//!
//! Themes are shared as `.comilito-theme` packages: a zip with `theme.json` at the top and
//! the files it names (e.g. `watermark.png`, `fonts/NewRocker.woff2`). Plain `.json` theme
//! files from older versions still import.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::storage;

/// The theme inside a `.comilito-theme` package.
const PACKAGE_THEME_FILE: &str = "theme.json";
/// Size limits, so a shared theme can't fill the disk or memory ("zip bomb"). They're checked
/// against the bytes actually read, not the sizes the zip claims.
const MAX_THEME_JSON: u64 = 256 * 1024;
const MAX_ASSET: u64 = 4 * 1024 * 1024;
const MAX_PACKAGE: u64 = 16 * 1024 * 1024;
const MAX_FONT_FILES: usize = 8;
const MAX_LICENCES: usize = 8;

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
    /// A woodcut drawn instead of the skeletal horse. Only its shape is used (its alpha);
    /// the app paints it in the theme's gold.
    ///
    /// `#[serde(default)]` means a file without this field still loads (as `None`), and
    /// `skip_serializing_if` leaves it out when empty, so older files stay byte-for-byte the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watermark: Option<String>,
    /// Font family → font file, for fonts that come with the theme. `fonts.display` or
    /// `fonts.body` can then name one of these families.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub font_files: BTreeMap<String, String>,
    /// Licence texts for what the theme bundles (e.g. a font's OFL.txt), kept so they travel
    /// with the theme when it's shared again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licences: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeFonts {
    pub display: String,
    pub body: String,
}

/// What a bundled file is for. Each role accepts only its own file types.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    Watermark,
    Font,
    Licence,
}

impl Role {
    fn accepts(self, ext: &str) -> bool {
        match self {
            Role::Watermark => matches!(ext, "png" | "webp" | "svg"),
            Role::Font => matches!(ext, "woff2" | "woff" | "ttf" | "otf"),
            Role::Licence => ext == "txt",
        }
    }
}

impl Theme {
    /// Every bundled file the theme refers to, with what it's for.
    fn files(&self) -> Vec<(&str, Role)> {
        let mut files = Vec::new();
        if let Some(w) = &self.watermark {
            files.push((w.as_str(), Role::Watermark));
        }
        files.extend(self.font_files.values().map(|f| (f.as_str(), Role::Font)));
        files.extend(self.licences.iter().map(|l| (l.as_str(), Role::Licence)));
        files
    }

    /// Replaces every file reference with `rename(old, role)`. Used when packing (hash →
    /// package path) and unpacking (package path → hash). `?` stops at the first error.
    fn rename_files(&mut self, mut rename: impl FnMut(&str, Role) -> Result<String, String>) -> Result<(), String> {
        if let Some(w) = &self.watermark {
            self.watermark = Some(rename(w, Role::Watermark)?);
        }
        for file in self.font_files.values_mut() {
            *file = rename(file, Role::Font)?;
        }
        for licence in self.licences.iter_mut() {
            *licence = rename(licence, Role::Licence)?;
        }
        Ok(())
    }
}

fn themes_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("themes")
}

fn assets_dir(config_dir: &Path) -> PathBuf {
    themes_dir(config_dir).join("assets")
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

/// The lower-case extension of a file name or package path ("fonts/A.WOFF2" → "woff2").
fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// A stored file's name: 64 lower-case hex digits (its SHA-256), a dot and an extension.
/// Nothing else is accepted, so a theme file can't point at `../../.bashrc`.
fn is_asset_name(name: &str) -> bool {
    let Some((hash, ext)) = name.split_once('.') else {
        return false;
    };
    hash.len() == 64
        && hash.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        && [Role::Watermark, Role::Font, Role::Licence].iter().any(|r| r.accepts(ext))
}

/// Checks a file's first bytes ("magic numbers") match its extension, so a renamed HTML page
/// or program is refused even if it's called `watermark.png`.
fn content_matches(ext: &str, bytes: &[u8]) -> bool {
    match ext {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "webp" => bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        // An SVG is only ever shown as an image (CSS mask), where scripts in it don't run.
        "svg" => std::str::from_utf8(bytes).is_ok_and(|text| text.contains("<svg")),
        "woff2" => bytes.starts_with(b"wOF2"),
        "woff" => bytes.starts_with(b"wOFF"),
        "ttf" => bytes.starts_with(&[0, 1, 0, 0]) || bytes.starts_with(b"true"),
        "otf" => bytes.starts_with(b"OTTO"),
        "txt" => std::str::from_utf8(bytes).is_ok(),
        _ => false,
    }
}

fn validate(theme: &Theme) -> Result<(), String> {
    if theme.name.trim().is_empty() {
        return Err("the theme needs a name".into());
    }
    if let Some((key, value)) = theme.colors.iter().find(|(_, v)| !is_hex_colour(v)) {
        return Err(format!("\"{key}\" is not a colour: {value}"));
    }
    let families = theme.font_files.keys().map(String::as_str);
    if ![theme.fonts.display.as_str(), theme.fonts.body.as_str()]
        .into_iter()
        .chain(families)
        .all(is_safe_font)
    {
        return Err("a font name has unsupported characters".into());
    }
    if theme.font_files.len() > MAX_FONT_FILES || theme.licences.len() > MAX_LICENCES {
        return Err("the theme brings too many files".into());
    }
    for (name, role) in theme.files() {
        if !is_asset_name(name) || !role.accepts(&extension(name)) {
            return Err(format!("the theme refers to a file it can't use: {name}"));
        }
    }
    Ok(())
}

/// Every theme file that parses, valid or not, with where it came from.
fn read_all(config_dir: &Path) -> Vec<Theme> {
    let Ok(entries) = fs::read_dir(themes_dir(config_dir)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|text| serde_json::from_str::<Theme>(&text).ok())
        .collect()
}

/// Every valid user theme, sorted by name. Unreadable files are skipped.
pub fn list(config_dir: &Path) -> Vec<Theme> {
    let mut themes: Vec<Theme> = read_all(config_dir)
        .into_iter()
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
    if let Some((missing, _)) = theme.files().into_iter().find(|(name, _)| !assets_dir(config_dir).join(name).is_file()) {
        return Err(format!("a file the theme uses is missing: {missing}"));
    }
    fs::create_dir_all(themes_dir(config_dir)).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(theme).map_err(|e| e.to_string())?;
    storage::write_atomically(&path_for(config_dir, &theme.id), json.as_bytes())
}

pub fn delete(config_dir: &Path, id: &str) -> Result<(), String> {
    let path = path_for(config_dir, id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("could not delete the theme: {e}"))?;
    }
    remove_unused_assets(config_dir);
    Ok(())
}

/// Deletes stored files that no theme refers to any more. Best effort: a file that can't be
/// removed now is tried again next time.
fn remove_unused_assets(config_dir: &Path) {
    // Owned `String`s, because the themes they come from are dropped at the end of this line.
    let used: HashSet<String> = read_all(config_dir)
        .iter()
        .flat_map(|t| t.files().into_iter().map(|(name, _)| name.to_string()))
        .collect();
    let Ok(entries) = fs::read_dir(assets_dir(config_dir)) else {
        return;
    };
    for entry in entries.flatten() {
        if !used.contains(&*entry.file_name().to_string_lossy()) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The bytes of a stored file, for the webview (fonts and the watermark).
pub fn read_asset(config_dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    if !is_asset_name(name) {
        return Err(format!("not a theme file: {name}"));
    }
    fs::read(assets_dir(config_dir).join(name)).map_err(|e| format!("could not read the theme file {name}: {e}"))
}

/// Stores a bundled file under its hash and returns that name. Storing the same file twice is
/// harmless: it gets the same name.
fn store_asset(config_dir: &Path, ext: &str, bytes: &[u8]) -> Result<String, String> {
    let hash: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    let name = format!("{hash}.{ext}");
    let dir = assets_dir(config_dir);
    let path = dir.join(&name);
    if !path.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        storage::write_atomically(&path, bytes)?;
    }
    Ok(name)
}

/// Reads one file out of a package, refusing it if it's bigger than `limit`.
fn read_entry<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>, path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let entry = archive
        .by_name(path)
        .map_err(|_| format!("the theme refers to {path}, but it isn't in the package"))?;
    let mut bytes = Vec::new();
    // `take` stops reading after `limit + 1` bytes whatever the zip claims, so we can tell
    // "exactly at the limit" from "too big" without decompressing the whole thing.
    entry
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("could not read {path}: {e}"))?;
    if bytes.len() as u64 > limit {
        return Err(format!("{path} is too big for a theme (the limit is {} MB)", limit / (1024 * 1024)));
    }
    Ok(bytes)
}

/// Reads a theme someone shared: a `.comilito-theme` package, or a plain `.json` theme.
/// It always gets a fresh id, so importing never overwrites an existing theme.
pub fn import(config_dir: &Path, file: &Path) -> Result<Theme, String> {
    let size = fs::metadata(file).map_err(|e| format!("could not read {}: {e}", file.display()))?.len();
    if size > MAX_PACKAGE {
        return Err("that file is too big to be a theme".into());
    }
    let data = fs::read(file).map_err(|e| format!("could not read {}: {e}", file.display()))?;
    let not_a_theme = || "that file isn't a Comilito theme".to_string();

    // Zip files start with "PK" (the initials of its inventor, Phil Katz).
    let mut theme = if data.starts_with(b"PK") {
        let mut archive = ZipArchive::new(Cursor::new(data)).map_err(|_| not_a_theme())?;
        let json = read_entry(&mut archive, PACKAGE_THEME_FILE, MAX_THEME_JSON)?;
        let mut theme: Theme = serde_json::from_slice(&json).map_err(|_| not_a_theme())?;
        let mut total = json.len() as u64;
        // Each package path is looked up by name and its bytes stored under a hash, so no
        // path from the zip is ever used to write to disk ("zip slip" can't happen).
        theme.rename_files(|path, role| {
            let ext = extension(path);
            if !role.accepts(&ext) {
                return Err(format!("{path} isn't a file type a theme can use there"));
            }
            let bytes = read_entry(&mut archive, path, MAX_ASSET)?;
            total += bytes.len() as u64;
            if total > MAX_PACKAGE {
                return Err("the theme's files are too big altogether".into());
            }
            if !content_matches(&ext, &bytes) {
                return Err(format!("{path} isn't really a .{ext} file"));
            }
            store_asset(config_dir, &ext, &bytes)
        })?;
        theme
    } else {
        let theme: Theme = serde_json::from_slice(&data).map_err(|_| not_a_theme())?;
        if !theme.files().is_empty() {
            return Err("this theme uses files that aren't in it; import its .comilito-theme package instead".into());
        }
        theme
    };
    theme.id = format!("user-{}", uuid::Uuid::new_v4().simple());
    save(config_dir, &theme)?;
    Ok(theme)
}

/// Writes a `.comilito-theme` package: `theme.json` plus the files it uses, under readable
/// names (`watermark.png`, `fonts/<family>.woff2`, `licences/licence-1.txt`).
pub fn export(config_dir: &Path, theme: &Theme, dest: &Path) -> Result<(), String> {
    validate(theme)?;
    let err = |e: &dyn std::fmt::Display| format!("could not write {}: {e}", dest.display());

    // Pair each stored file with its path in the package, and rewrite the theme to match.
    let mut packed = theme.clone();
    let mut files: Vec<(String, String)> = Vec::new(); // (package path, stored name)
    packed.rename_files(|name, role| {
        let ext = extension(name);
        let path = match role {
            Role::Watermark => format!("watermark.{ext}"),
            Role::Font => {
                // Named after its family ("New Rocker" → fonts/NewRocker.woff2).
                let family = theme
                    .font_files
                    .iter()
                    .find(|(_, file)| file.as_str() == name)
                    .map(|(family, _)| family.replace(' ', ""))
                    .unwrap_or_default();
                format!("fonts/{}.{ext}", storage::safe_file_name(&family))
            }
            Role::Licence => format!("licences/licence-{}.txt", files.iter().filter(|(p, _)| p.starts_with("licences/")).count() + 1),
        };
        // Two roles can share a stored file (the same font for two families); pack it once.
        if !files.iter().any(|(p, _)| p == &path) {
            files.push((path.clone(), name.to_string()));
        }
        Ok(path)
    })?;
    packed.id = String::new(); // importing always assigns a new id

    let tmp = dest.with_extension("comilito-theme.tmp");
    let mut zip = ZipWriter::new(fs::File::create(&tmp).map_err(|e| err(&e))?);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let json = serde_json::to_vec_pretty(&packed).map_err(|e| err(&e))?;
    zip.start_file(PACKAGE_THEME_FILE, deflated).map_err(|e| err(&e))?;
    zip.write_all(&json).map_err(|e| err(&e))?;
    for (path, name) in &files {
        let bytes = read_asset(config_dir, name)?;
        zip.start_file(path.as_str(), deflated).map_err(|e| err(&e))?;
        zip.write_all(&bytes).map_err(|e| err(&e))?;
    }
    zip.finish().map_err(|e| err(&e))?;
    fs::rename(&tmp, dest).map_err(|e| err(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfake image";
    const WOFF2: &[u8] = b"wOF2fake font";

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
            watermark: None,
            font_files: BTreeMap::new(),
            licences: Vec::new(),
        }
    }

    /// A package as a theme author would zip it by hand.
    fn write_package(path: &Path, theme_json: &str, files: &[(&str, &[u8])]) {
        let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
        zip.start_file(PACKAGE_THEME_FILE, SimpleFileOptions::default()).unwrap();
        zip.write_all(theme_json.as_bytes()).unwrap();
        for (name, bytes) in files {
            zip.start_file(*name, SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }

    const PACKAGED: &str = r##"{
        "id": "", "name": "Horde", "dark": true,
        "colors": { "bg": "#1c1412" },
        "fonts": { "display": "New Rocker", "body": "Inter" },
        "watermark": "watermark.png",
        "fontFiles": { "New Rocker": "fonts/NewRocker.woff2" },
        "licences": ["fonts/OFL.txt"]
    }"##;

    #[test]
    fn saves_lists_imports_and_deletes() {
        let dir = TempDir::new();
        save(&dir.0, &theme("user-1")).unwrap();
        assert_eq!(list(&dir.0).len(), 1);

        let file = dir.0.join("shared.comilito-theme");
        export(&dir.0, &theme("user-1"), &file).unwrap();
        let imported = import(&dir.0, &file).unwrap();
        assert_ne!(imported.id, "user-1", "imports get a new id");
        assert_eq!(list(&dir.0).len(), 2);

        delete(&dir.0, "user-1").unwrap();
        assert_eq!(list(&dir.0).len(), 1);
    }

    #[test]
    fn plain_json_themes_still_import() {
        let dir = TempDir::new();
        let file = dir.0.join("old.json");
        fs::write(&file, serde_json::to_string(&theme("user-9")).unwrap()).unwrap();
        assert_eq!(import(&dir.0, &file).unwrap().name, "My Tavern");
    }

    #[test]
    fn imports_a_package_and_exports_it_again() {
        let dir = TempDir::new();
        let file = dir.0.join("horde.comilito-theme");
        write_package(&file, PACKAGED, &[("watermark.png", PNG), ("fonts/NewRocker.woff2", WOFF2), ("fonts/OFL.txt", b"OFL")]);

        let imported = import(&dir.0, &file).unwrap();
        let watermark = imported.watermark.clone().unwrap();
        assert!(is_asset_name(&watermark) && watermark.ends_with(".png"));
        assert_eq!(read_asset(&dir.0, &watermark).unwrap(), PNG);
        assert_eq!(read_asset(&dir.0, &imported.font_files["New Rocker"]).unwrap(), WOFF2);

        // Export, wipe, import again: the same files come back.
        let again = dir.0.join("again.comilito-theme");
        export(&dir.0, &imported, &again).unwrap();
        delete(&dir.0, &imported.id).unwrap();
        assert!(read_asset(&dir.0, &watermark).is_err(), "unused files are cleaned up");
        let reimported = import(&dir.0, &again).unwrap();
        assert_eq!(reimported.watermark, Some(watermark));
        assert_eq!(reimported.licences.len(), 1);
    }

    #[test]
    fn a_copy_keeps_shared_files_when_the_original_is_deleted() {
        let dir = TempDir::new();
        let file = dir.0.join("horde.comilito-theme");
        write_package(&file, PACKAGED, &[("watermark.png", PNG), ("fonts/NewRocker.woff2", WOFF2), ("fonts/OFL.txt", b"OFL")]);
        let original = import(&dir.0, &file).unwrap();
        let copy = Theme { id: "user-copy".into(), ..original.clone() };
        save(&dir.0, &copy).unwrap();

        delete(&dir.0, &original.id).unwrap();
        assert!(read_asset(&dir.0, copy.watermark.as_deref().unwrap()).is_ok());
    }

    #[test]
    fn refuses_bad_packages() {
        let dir = TempDir::new();
        let file = dir.0.join("bad.comilito-theme");

        // A "PNG" that's really something else.
        write_package(&file, PACKAGED, &[("watermark.png", b"<html>"), ("fonts/NewRocker.woff2", WOFF2), ("fonts/OFL.txt", b"OFL")]);
        assert!(import(&dir.0, &file).is_err());

        // A file the theme names but the package lacks.
        write_package(&file, PACKAGED, &[("watermark.png", PNG)]);
        assert!(import(&dir.0, &file).is_err());

        // A font file used as the watermark.
        let wrong_role = PACKAGED.replace("\"watermark.png\"", "\"fonts/NewRocker.woff2\"");
        write_package(&file, &wrong_role, &[("fonts/NewRocker.woff2", WOFF2), ("fonts/OFL.txt", b"OFL")]);
        assert!(import(&dir.0, &file).is_err());

        // A plain JSON theme that points at files.
        let json = dir.0.join("loose.json");
        fs::write(&json, PACKAGED).unwrap();
        assert!(import(&dir.0, &json).is_err());

        assert!(list(&dir.0).is_empty());
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
        let mut escaping = theme("user-4");
        escaping.watermark = Some("../../.bashrc".into());
        assert!(save(&dir.0, &escaping).is_err());
        let mut missing = theme("user-5");
        missing.watermark = Some(format!("{}.png", "a".repeat(64)));
        assert!(save(&dir.0, &missing).is_err(), "files must exist");
        assert!(read_asset(&dir.0, "../prefs.json").is_err());
    }
}
