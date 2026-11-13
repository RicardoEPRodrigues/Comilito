//! Project folders on disk: list, create, load, save, and copy audio in.
//! See docs/save-format.md and decision 0006.
//!
//! Every function takes the folder it works on as a parameter instead of looking it up
//! itself. That keeps this module free of Tauri and easy to test against a temporary folder.
//!
//! Errors are plain `String`s with a message for the user. A bigger app would define its
//! own error enum; for us a readable message is all the UI needs.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::project::{Category, FORMAT_VERSION, FileInfo, Project, ProjectDefaults};

pub const PROJECT_FILE: &str = "comilito.json";

/// What the project picker needs to show.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub name: String,
    pub path: PathBuf,
}

/// Finds every folder directly inside `root` that contains a `comilito.json`.
pub fn list_projects(root: &Path) -> Vec<ProjectSummary> {
    // `let Ok(x) = ... else { ... }` is "unwrap this or bail out": if reading the folder
    // fails (for example it doesn't exist yet), there are simply no projects.
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut projects: Vec<ProjectSummary> = entries
        // `flatten()` skips entries that failed to read (each item is a Result).
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(PROJECT_FILE).is_file())
        .map(|path| {
            // Show the name stored in the file; fall back to the folder name if it can't be read.
            let name = load_project(&path)
                .map(|p| p.name)
                .unwrap_or_else(|_| folder_name(&path));
            ProjectSummary { name, path }
        })
        .collect();

    projects.sort_by_key(|p| p.name.to_lowercase());
    projects
}

/// Creates `root/<name>/` with an empty project and its audio folders.
pub fn create_project(
    root: &Path,
    name: &str,
    defaults: ProjectDefaults,
) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("the project needs a name".into());
    }

    let dir = root.join(safe_file_name(name));
    if dir.exists() {
        return Err(format!("a folder named \"{}\" already exists", folder_name(&dir)));
    }

    create_audio_folders(&dir)?;
    save_project(&dir, &Project::new(name, defaults))?;
    Ok(dir)
}

pub fn create_audio_folders(dir: &Path) -> Result<(), String> {
    for category in Category::ALL {
        let sub = dir.join("audio").join(category.folder_name());
        fs::create_dir_all(&sub).map_err(|e| format!("could not create {}: {e}", sub.display()))?;
    }
    Ok(())
}

/// Reads `dir/comilito.json`.
pub fn load_project(dir: &Path) -> Result<Project, String> {
    let path = dir.join(PROJECT_FILE);
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    parse_project(&text)
}

/// Parses the JSON text of a project, checking its format version first.
///
/// To add a format version 2 later: read the text as a generic `serde_json::Value`, and if
/// `formatVersion` is 1, change the JSON to the new shape before turning it into a `Project`.
/// Old files then keep loading forever (a rule in CLAUDE.md).
pub fn parse_project(text: &str) -> Result<Project, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("comilito.json is not valid JSON: {e}"))?;

    let version = value
        .get("formatVersion")
        .and_then(|v| v.as_u64())
        .ok_or("comilito.json has no formatVersion")?;
    if version > FORMAT_VERSION as u64 {
        return Err(format!(
            "this project was saved by a newer version of Comilito (format {version}); please update the app"
        ));
    }

    let mut project: Project = serde_json::from_value(value)
        .map_err(|e| format!("comilito.json has an unexpected shape: {e}"))?;
    project.format_version = FORMAT_VERSION;
    project.remove_dangling_references();
    Ok(project)
}

/// Writes `dir/comilito.json` atomically: the new content goes to a temporary file, which
/// is then renamed over the old one. A rename is atomic on the same disk, so a sync tool
/// (or a crash) never sees a half-written file.
pub fn save_project(dir: &Path, project: &Project) -> Result<(), String> {
    let json = serde_json::to_string_pretty(project)
        .map_err(|e| format!("could not serialize project: {e}"))?;
    write_atomically(&dir.join(PROJECT_FILE), json.as_bytes())
}

pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    // A block `{ ... }` limits how long `file` lives. When it ends, `file` is dropped and the
    // OS file handle is closed (Rust's version of RAII), before we rename it.
    {
        let mut file = fs::File::create(&tmp)
            .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("could not write {}: {e}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("could not replace {}: {e}", path.display()))
}

/// The modification time of `comilito.json`, to notice when something else (a sync tool)
/// changed it.
pub fn project_file_modified(dir: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(dir.join(PROJECT_FILE)).and_then(|m| m.modified()).ok()
}

/// Copies an audio file into `dir/audio/<category>/` and returns its `FileInfo`.
/// If a file with that name is already there, a number is added (`rain (2).ogg`).
pub fn copy_audio_into_project(
    dir: &Path,
    category: Category,
    source: &Path,
) -> Result<FileInfo, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("{} is not a file", source.display()))?;
    let target_dir = dir.join("audio").join(category.folder_name());
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("could not create {}: {e}", target_dir.display()))?;

    let target = unique_path(&target_dir, Path::new(file_name));
    fs::copy(source, &target).map_err(|e| format!("could not copy {}: {e}", source.display()))?;

    let relative = target
        .strip_prefix(dir)
        .expect("target was built inside dir");
    file_info(dir, relative)
}

/// Moves a sound's file into `audio/<category>/` (renamed if the name is taken there) and
/// returns its updated `FileInfo`. The hash is kept: the content doesn't change.
pub fn move_audio_file(dir: &Path, info: &FileInfo, category: Category) -> Result<FileInfo, String> {
    let source = absolute_audio_path(dir, info);
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("{} is not a file", source.display()))?;
    let target_dir = dir.join("audio").join(category.folder_name());
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("could not create {}: {e}", target_dir.display()))?;
    let target = unique_path(&target_dir, Path::new(file_name));
    fs::rename(&source, &target)
        .map_err(|e| format!("could not move {}: {e}", source.display()))?;

    let relative = target.strip_prefix(dir).expect("target was built inside dir");
    let mut moved = file_info(dir, relative)?;
    moved.sha256 = info.sha256.clone();
    moved.loudness = info.loudness;
    Ok(moved)
}

/// Size and modification time of a file inside the project. `sha256` is filled in later
/// by the background hasher.
pub fn file_info(dir: &Path, relative: &Path) -> Result<FileInfo, String> {
    let absolute = dir.join(relative);
    let meta = fs::metadata(&absolute)
        .map_err(|e| format!("could not read {}: {e}", absolute.display()))?;
    Ok(FileInfo {
        path: to_json_path(relative),
        size: meta.len(),
        modified: meta.modified().map(format_time).unwrap_or_default(),
        sha256: None,
        loudness: None,
    })
}

/// Turns a stored relative path into a real one for this OS.
pub fn absolute_audio_path(dir: &Path, info: &FileInfo) -> PathBuf {
    // Paths are stored with `/`. `Path::join` on each part gives `\` on Windows.
    info.path.split('/').fold(dir.to_path_buf(), |p, part| p.join(part))
}

/// File extensions treated as audio when scanning a project folder (what kira can decode).
pub const AUDIO_EXTENSIONS: [&str; 5] = ["mp3", "ogg", "oga", "flac", "wav"];

/// An audio file found in a project's `audio/<category>/` folder.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundAudio {
    /// Relative to the project folder, with `/`, as stored in comilito.json.
    pub path: String,
    pub category: Category,
    pub size: u64,
}

/// Every audio file under `dir/audio/<category>/` (subfolders included). Hidden files are
/// skipped: sync tools and editors use them for temporary copies.
pub fn scan_audio_files(dir: &Path) -> Vec<FoundAudio> {
    let mut found = Vec::new();
    for category in Category::ALL {
        let mut to_visit = vec![dir.join("audio").join(category.folder_name())];
        while let Some(folder) = to_visit.pop() {
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let Ok(meta) = entry.metadata() else { continue };
                if meta.is_dir() {
                    to_visit.push(path);
                    continue;
                }
                let is_audio = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.as_str()));
                if !is_audio {
                    continue;
                }
                let Ok(relative) = path.strip_prefix(dir) else { continue };
                found.push(FoundAudio {
                    path: to_json_path(relative),
                    category,
                    size: meta.len(),
                });
            }
        }
    }
    found
}

/// Ids of sounds whose audio file is missing from the project folder.
pub fn missing_sounds(dir: &Path, project: &Project) -> Vec<String> {
    project
        .sounds
        .iter()
        .filter(|s| !absolute_audio_path(dir, &s.file).is_file())
        .map(|s| s.id.clone())
        .collect()
}

pub fn format_time(time: std::time::SystemTime) -> String {
    let utc: DateTime<Utc> = time.into();
    utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Stored paths always use `/`, whatever the OS.
fn to_json_path(relative: &Path) -> String {
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// `dir/name` if free, otherwise `dir/name (2).ext`, `dir/name (3).ext`...
pub fn unique_path(dir: &Path, name: &Path) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let stem = name.file_stem().unwrap_or_default().to_string_lossy();
    let ext = name
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    // `(2..)` is an endless range; the loop always returns before running out.
    for n in 2.. {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

/// Removes characters that aren't allowed in file names on Windows (the strictest OS),
/// so a project folder made on Linux can be synced to Windows.
pub fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    // Windows also refuses names ending in a dot or space.
    cleaned.trim().trim_end_matches('.').to_string()
}

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
pub mod test_support {
    use std::path::PathBuf;

    /// A fresh empty folder under the system temp dir, removed when dropped.
    /// (A tiny stand-in for the `tempfile` crate, to avoid a dependency for tests.)
    pub struct TempDir(pub PathBuf);

    impl TempDir {
        pub fn new() -> Self {
            let path = std::env::temp_dir().join(format!("comilito-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
    }

    // `Drop` is Rust's destructor: it runs automatically when the value goes out of scope.
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::TempDir;
    use super::*;

    #[test]
    fn create_list_load_round_trip() {
        let root = TempDir::new();
        let dir = create_project(&root.0, "Curse of Strahd", ProjectDefaults::default()).unwrap();

        assert!(dir.join("audio/music").is_dir());
        let listed = list_projects(&root.0);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Curse of Strahd");

        let project = load_project(&dir).unwrap();
        assert_eq!(project.name, "Curse of Strahd");
    }

    #[test]
    fn refuses_duplicate_and_empty_names() {
        let root = TempDir::new();
        create_project(&root.0, "A", ProjectDefaults::default()).unwrap();
        assert!(create_project(&root.0, "A", ProjectDefaults::default()).is_err());
        assert!(create_project(&root.0, "  ", ProjectDefaults::default()).is_err());
    }

    #[test]
    fn refuses_newer_format_versions() {
        let err = parse_project(r#"{"formatVersion": 99}"#).unwrap_err();
        assert!(err.contains("newer version"), "{err}");
    }

    #[test]
    fn copies_audio_with_unique_names_and_relative_paths() {
        let root = TempDir::new();
        let dir = create_project(&root.0, "P", ProjectDefaults::default()).unwrap();
        let source = root.0.join("rain.ogg");
        fs::write(&source, b"fake audio").unwrap();

        let first = copy_audio_into_project(&dir, Category::Ambience, &source).unwrap();
        let second = copy_audio_into_project(&dir, Category::Ambience, &source).unwrap();

        assert_eq!(first.path, "audio/ambience/rain.ogg");
        assert_eq!(second.path, "audio/ambience/rain (2).ogg");
        assert_eq!(first.size, 10);
        assert!(absolute_audio_path(&dir, &second).is_file());
    }

    #[test]
    fn moves_audio_between_category_folders() {
        let root = TempDir::new();
        let dir = create_project(&root.0, "P", ProjectDefaults::default()).unwrap();
        let source = root.0.join("bell.ogg");
        fs::write(&source, b"ding").unwrap();
        let mut info = copy_audio_into_project(&dir, Category::Music, &source).unwrap();
        info.sha256 = Some("abc".into());

        let moved = move_audio_file(&dir, &info, Category::Effects).unwrap();
        assert_eq!(moved.path, "audio/effects/bell.ogg");
        assert_eq!(moved.sha256.as_deref(), Some("abc"));
        assert!(absolute_audio_path(&dir, &moved).is_file());
        assert!(!absolute_audio_path(&dir, &info).exists());
    }

    #[test]
    fn scans_audio_files_by_category_folder() {
        let root = TempDir::new();
        let dir = create_project(&root.0, "P", ProjectDefaults::default()).unwrap();
        fs::create_dir_all(dir.join("audio/ambience/forest")).unwrap();
        fs::write(dir.join("audio/music/theme.MP3"), b"123").unwrap();
        fs::write(dir.join("audio/ambience/forest/birds.ogg"), b"12345").unwrap();
        fs::write(dir.join("audio/effects/notes.txt"), b"not audio").unwrap();
        fs::write(dir.join("audio/effects/.door.wav.part"), b"temp").unwrap();
        fs::write(dir.join("audio/effects/.hidden.wav"), b"temp").unwrap();

        let mut found = scan_audio_files(&dir);
        found.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(
            found,
            vec![
                FoundAudio { path: "audio/ambience/forest/birds.ogg".into(), category: Category::Ambience, size: 5 },
                FoundAudio { path: "audio/music/theme.MP3".into(), category: Category::Music, size: 3 },
            ]
        );
    }

    #[test]
    fn makes_names_safe_for_windows() {
        assert_eq!(safe_file_name("Act 1: The Lich?"), "Act 1- The Lich-");
        assert_eq!(safe_file_name("trailing. "), "trailing");
    }
}
