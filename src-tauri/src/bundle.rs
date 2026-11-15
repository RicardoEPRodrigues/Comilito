//! `.comilito` zips (export, import, backups), file hashing and finding missing audio.
//! See docs/save-format.md and decisions 0006 and 0007.
//!
//! A `.comilito` file is the project folder zipped: `comilito.json` plus, optionally,
//! `audio/...`. Audio is stored uncompressed (it's already compressed); the JSON is deflated.

use std::{
    fs::{self, File},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::{
    project::{Category, Project},
    storage::{self, PROJECT_FILE},
};

/// Writes `project` (and, if asked, its audio and its videos) to a `.comilito` zip at `dest`.
/// Videos are asked about separately: they're much bigger (decision 0035).
pub fn export_zip(dir: &Path, project: &Project, dest: &Path, include_audio: bool, include_video: bool) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("could not write {}: {e}", dest.display());

    // Write to a temporary name first, so a failed export never leaves a broken file
    // where the user expects a good one.
    let tmp = dest.with_extension("comilito.tmp");
    let file = File::create(&tmp).map_err(|e| err(&e))?;
    let mut zip = ZipWriter::new(file);

    let json = serde_json::to_vec_pretty(project).map_err(|e| err(&e))?;
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(PROJECT_FILE, deflated).map_err(|e| err(&e))?;
    zip.write_all(&json).map_err(|e| err(&e))?;

    // Media files are already compressed, so they're stored as they are. `large_file` keeps
    // the zip valid for files over 4 GB (long videos).
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored).large_file(true);
    let audio = project.sounds.iter().map(|s| &s.file).filter(|_| include_audio);
    let video = project.videos.iter().map(|v| &v.file).filter(|_| include_video);
    for file in audio.chain(video) {
        let path = storage::absolute_audio_path(dir, file);
        // A missing file is skipped, not fatal: export what we have.
        let Ok(mut source) = File::open(&path) else {
            continue;
        };
        zip.start_file(&file.path, stored).map_err(|e| err(&e))?;
        io::copy(&mut source, &mut zip).map_err(|e| err(&e))?;
    }

    zip.finish().map_err(|e| err(&e))?;
    fs::rename(&tmp, dest).map_err(|e| err(&e))
}

/// Reads only the project out of a `.comilito` zip (also used to check a backup).
pub fn read_zip_project(zip_path: &Path) -> Result<Project, String> {
    let file = File::open(zip_path).map_err(|e| format!("could not open {}: {e}", zip_path.display()))?;
    let mut archive = ZipArchive::new(BufReader::new(file))
        .map_err(|e| format!("{} is not a valid .comilito file: {e}", zip_path.display()))?;
    let mut entry = archive
        .by_name(PROJECT_FILE)
        .map_err(|_| format!("{} has no {PROJECT_FILE} inside", zip_path.display()))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("could not read {PROJECT_FILE}: {e}"))?;
    storage::parse_project(&text)
}

/// Unpacks a `.comilito` zip into a new project folder under `root` (renamed if the name is
/// taken) and returns that folder.
pub fn import_zip(zip_path: &Path, root: &Path) -> Result<PathBuf, String> {
    let project = read_zip_project(zip_path)?;

    fs::create_dir_all(root).map_err(|e| format!("could not create {}: {e}", root.display()))?;
    let dir = storage::unique_path(root, Path::new(&storage::safe_file_name(&project.name)));
    storage::create_audio_folders(&dir)?;

    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(BufReader::new(file)).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // `enclosed_name` refuses paths like `../../.bashrc` or `/etc/passwd`, so a malicious
        // zip can't write outside the new folder ("zip slip").
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        // Only take what belongs in a project.
        let wanted = relative == Path::new(PROJECT_FILE) || relative.starts_with("audio") || relative.starts_with("video");
        if !wanted || entry.is_dir() {
            continue;
        }
        let target = dir.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = File::create(&target).map_err(|e| format!("could not write {}: {e}", target.display()))?;
        io::copy(&mut entry, &mut out).map_err(|e| format!("could not write {}: {e}", target.display()))?;
    }
    Ok(dir)
}

/// Writes a backup zip to `backups_dir/<project name>/<project name> YYYY-MM-DD HH-MM.comilito`,
/// checks it can be read back, and only then deletes the oldest backups beyond `keep`.
/// There is never a moment with zero backups (decision 0007).
pub fn write_backup(dir: &Path, project: &Project, backups_dir: &Path) -> Result<PathBuf, String> {
    let name = storage::safe_file_name(&project.name);
    let folder = backups_dir.join(&name);
    fs::create_dir_all(&folder).map_err(|e| format!("could not create {}: {e}", folder.display()))?;

    let stamp = chrono::Local::now().format("%Y-%m-%d %H-%M");
    let dest = storage::unique_path(&folder, Path::new(&format!("{name} {stamp}.comilito")));
    export_zip(dir, project, &dest, project.backups.include_audio, project.backups.include_video)?;
    read_zip_project(&dest).map_err(|e| format!("the new backup didn't verify: {e}"))?;

    // Oldest first, by modification time. (Not by name: two backups in the same minute are
    // named "X 10-30.comilito" and "X 10-30 (2).comilito", which sort the wrong way round.)
    let mut backups: Vec<(std::time::SystemTime, PathBuf)> = fs::read_dir(&folder)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "comilito"))
        .map(|p| {
            let modified = fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            (modified, p)
        })
        .collect();
    backups.sort();
    let keep = project.backups.keep.max(1) as usize;
    let excess = backups.len().saturating_sub(keep);
    for (_, old) in &backups[..excess] {
        // Never delete the backup we just wrote, even if clocks are odd.
        if old != &dest {
            let _ = fs::remove_file(old);
        }
    }
    Ok(dest)
}

/// SHA-256 of a file, as lowercase hex. Reads in chunks, so big files don't fill memory.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    // `{:02x}` prints each byte as two hex digits.
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// A file found while looking for a missing sound.
pub struct Candidate {
    pub path: PathBuf,
    pub size: u64,
}

/// Every file under `folder` (recursively) whose name is `file_name`.
pub fn find_by_name(folder: &Path, file_name: &str) -> Vec<Candidate> {
    let mut found = Vec::new();
    // An explicit stack of folders still to visit, instead of recursion.
    let mut to_visit = vec![folder.to_path_buf()];
    while let Some(current) = to_visit.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                to_visit.push(path);
            } else if entry.file_name().to_string_lossy() == file_name {
                found.push(Candidate { path, size: meta.len() });
            }
        }
    }
    found
}

/// Picks the best match for a missing file among same-named candidates: the same size wins,
/// and if several have that size, the one with the same hash (docs/save-format.md).
pub fn best_candidate(candidates: Vec<Candidate>, size: u64, sha256: Option<&str>) -> Option<PathBuf> {
    let same_size: Vec<Candidate> = candidates.into_iter().filter(|c| c.size == size).collect();
    match (same_size.len(), sha256) {
        (0, _) => None,
        (1, _) => Some(same_size.into_iter().next()?.path),
        (_, Some(hash)) => same_size
            .into_iter()
            .find(|c| sha256_file(&c.path).is_ok_and(|h| h == hash))
            .map(|c| c.path),
        // Several files with the same name and size and no hash to tell them apart: they're
        // very likely the same file anyway.
        (_, None) => Some(same_size.into_iter().next()?.path),
    }
}

/// The category folder a stored path lives in, used when copying a relinked file back in.
pub fn category_of_path(path: &str, fallback: Category) -> Category {
    Category::ALL
        .into_iter()
        .find(|c| path.starts_with(&format!("audio/{}/", c.folder_name())))
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{ProjectDefaults, Sound};
    use crate::storage::test_support::TempDir;

    /// A project folder with one sound whose (fake) audio file exists.
    fn project_with_sound(root: &Path) -> (PathBuf, Project) {
        let dir = storage::create_project(root, "Strahd", ProjectDefaults::default()).unwrap();
        let source = root.join("rain.ogg");
        fs::write(&source, b"fake rain audio").unwrap();
        let file = storage::copy_audio_into_project(&dir, Category::Ambience, "", &source).unwrap();
        let mut project = storage::load_project(&dir).unwrap();
        project.sounds.push(Sound {
            id: "snd-rain".into(),
            ..Sound::new(Category::Ambience, "Rain".into(), file)
        });
        storage::save_project(&dir, &project).unwrap();
        (dir, project)
    }

    #[test]
    fn export_then_import_with_audio() {
        let root = TempDir::new();
        let (dir, project) = project_with_sound(&root.0);
        let zip = root.0.join("out.comilito");
        export_zip(&dir, &project, &zip, true, true).unwrap();

        let other_root = TempDir::new();
        let imported = import_zip(&zip, &other_root.0).unwrap();
        let loaded = storage::load_project(&imported).unwrap();
        assert_eq!(loaded.sounds.len(), 1);
        assert!(storage::missing_sounds(&imported, &loaded).is_empty());
    }

    #[test]
    fn export_without_audio_imports_with_missing_files_and_renames_on_clash() {
        let root = TempDir::new();
        let (dir, project) = project_with_sound(&root.0);
        let zip = root.0.join("small.comilito");
        export_zip(&dir, &project, &zip, false, false).unwrap();

        // Importing next to the original: the folder name is taken, so it gets "(2)".
        let imported = import_zip(&zip, &root.0).unwrap();
        assert!(imported.ends_with("Strahd (2)"));
        let loaded = storage::load_project(&imported).unwrap();
        assert_eq!(storage::missing_sounds(&imported, &loaded), vec!["snd-rain"]);
    }

    #[test]
    fn videos_are_exported_only_when_asked() {
        let root = TempDir::new();
        let (dir, mut project) = project_with_sound(&root.0);
        std::fs::create_dir_all(dir.join("video/ambience/Castle")).unwrap();
        std::fs::write(dir.join("video/ambience/Castle/gate.webm"), b"video").unwrap();
        let file = storage::file_info(&dir, Path::new("video/ambience/Castle/gate.webm")).unwrap();
        project.videos.push(crate::video::new_video("Gate".into(), file, crate::project::VideoKind::Ambience));
        storage::save_project(&dir, &project).unwrap();

        let with = root.0.join("with.comilito");
        export_zip(&dir, &project, &with, true, true).unwrap();
        // Named, so the folder lives until the end of the test (a temporary `TempDir` would be
        // dropped, and its folder deleted, at the end of the statement).
        let target = TempDir::new();
        let imported = import_zip(&with, &target.0).unwrap();
        assert!(imported.join("video/ambience/Castle/gate.webm").is_file());

        let audio_only = root.0.join("audio-only.comilito");
        export_zip(&dir, &project, &audio_only, true, false).unwrap();
        let other = TempDir::new();
        let imported = import_zip(&audio_only, &other.0).unwrap();
        assert!(!imported.join("video/ambience/Castle/gate.webm").exists());
        let loaded = storage::load_project(&imported).unwrap();
        assert_eq!(crate::video::missing(&imported, &loaded).len(), 1, "the video is listed, its file missing");
    }

    #[test]
    fn backups_keep_only_the_newest() {
        let root = TempDir::new();
        let (dir, mut project) = project_with_sound(&root.0);
        project.backups.keep = 2;
        let backups = root.0.join("Backups");
        let mut written = Vec::new();
        for _ in 0..3 {
            written.push(write_backup(&dir, &project, &backups).unwrap());
            // Make sure modification times differ even on coarse filesystems.
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let count = fs::read_dir(backups.join("Strahd")).unwrap().count();
        assert_eq!(count, 2);
        assert!(!written[0].exists(), "the oldest is deleted");
        assert!(written[2].exists(), "the newest is kept");
    }

    #[test]
    fn finds_a_missing_file_by_name_and_size() {
        let root = TempDir::new();
        let nested = root.0.join("a/b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("rain.ogg"), b"12345").unwrap();
        fs::write(root.0.join("rain.ogg"), b"123").unwrap();

        let found = find_by_name(&root.0, "rain.ogg");
        assert_eq!(found.len(), 2);
        let best = best_candidate(found, 5, None).unwrap();
        assert!(best.ends_with("a/b/rain.ogg"));
    }

    #[test]
    fn hash_is_known_value() {
        let root = TempDir::new();
        let path = root.0.join("abc.txt");
        fs::write(&path, b"abc").unwrap();
        assert_eq!(
            sha256_file(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn category_from_stored_path() {
        assert_eq!(category_of_path("audio/music/x.ogg", Category::Effects), Category::Music);
        assert_eq!(category_of_path("elsewhere/x.ogg", Category::Effects), Category::Effects);
    }
}
