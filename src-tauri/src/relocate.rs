//! Moving the projects folder somewhere else, duplicating a project, and moving backups out
//! of the projects folder (where older versions kept them, decision 0026).
//!
//! Only Comilito's own things are moved: folders holding a `comilito.json`. The projects
//! folder may be a folder the user also keeps other files in (say `~/Documents`), and those
//! must stay where they are.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::storage::{self, PROJECT_FILE};

/// Where versions before decision 0026 kept backups: `<projects folder>/Backups`.
pub const OLD_BACKUPS: &str = "Backups";

/// What a move would carry: the project folders in `root`.
pub fn movable_entries(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join(PROJECT_FILE).is_file())
        .collect();
    found.sort();
    found
}

/// A checked move, ready to run (see `plan_move` and `run_move`). Checking is quick and
/// happens while the user waits; running can take minutes across disks, so it runs on a
/// background thread.
pub struct MovePlan {
    pub from: PathBuf,
    pub to: PathBuf,
    entries: Vec<PathBuf>,
    /// Bytes in all the entries, for the progress bar.
    pub total_bytes: u64,
}

/// Checks a move of every project folder from `from` into `to`. Checking
/// everything first means the move either does all of them or (barring a disk error halfway)
/// none: no name may already exist in `to`.
pub fn plan_move(from: &Path, to: &Path) -> Result<MovePlan, String> {
    if same_place(from, to) {
        return Err("that's already the projects folder".into());
    }
    let entries = movable_entries(from);
    // Moving into one of the folders being moved would move a folder into itself.
    if entries.iter().any(|e| to.starts_with(e)) {
        return Err("the new folder can't be inside a project".into());
    }
    fs::create_dir_all(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    for entry in &entries {
        let target = to.join(entry.file_name().expect("entries have names"));
        if target.exists() {
            return Err(format!(
                "{} already has something called \"{}\"; move or rename it first",
                to.display(),
                target.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }
    let total_bytes = entries.iter().map(|e| dir_size(e)).sum();
    Ok(MovePlan { from: from.to_path_buf(), to: to.to_path_buf(), entries, total_bytes })
}

/// Runs a planned move and returns how many folders moved. `progress(done_bytes, current)`
/// is called after each file (or at once for a folder that could simply be renamed).
pub fn run_move(plan: &MovePlan, progress: &mut dyn FnMut(u64, &Path)) -> Result<usize, String> {
    let mut done = 0u64;
    for (moved, entry) in plan.entries.iter().enumerate() {
        let target = plan.to.join(entry.file_name().expect("entries have names"));
        move_dir(entry, &target, &mut done, progress).map_err(|e| {
            format!("{e} ({moved} of {} moved; the rest are still in {})", plan.entries.len(), plan.from.display())
        })?;
    }
    // Leave no empty folder behind (remove_dir refuses folders that still hold something).
    let _ = fs::remove_dir(&plan.from);
    Ok(plan.entries.len())
}

/// Plans and runs a move in one go, without progress (used by the tests).
#[cfg(test)]
pub fn move_projects(from: &Path, to: &Path) -> Result<usize, String> {
    run_move(&plan_move(from, to)?, &mut |_, _| {})
}

/// Moves backups from `<projects folder>/Backups` (older versions) into `backups_dir`, and
/// returns how many files moved. Backups already in `backups_dir` win: a file with the same
/// name is left in the old folder rather than overwritten. The old folder is removed once empty.
pub fn migrate_backups(projects_root: &Path, backups_dir: &Path) -> Result<usize, String> {
    let old = projects_root.join(OLD_BACKUPS);
    if !old.is_dir() || same_place(&old, backups_dir) {
        return Ok(0);
    }
    let mut moved = 0;
    // Backups are laid out as `<project name>/<project name> <date>.comilito`.
    let projects = fs::read_dir(&old).map_err(|e| format!("could not read {}: {e}", old.display()))?;
    for project in projects.flatten().filter(|e| e.path().is_dir()) {
        let target_dir = backups_dir.join(project.file_name());
        fs::create_dir_all(&target_dir).map_err(|e| format!("could not create {}: {e}", target_dir.display()))?;
        for file in fs::read_dir(project.path()).into_iter().flatten().flatten() {
            let target = target_dir.join(file.file_name());
            if target.exists() || !file.path().is_file() {
                continue;
            }
            move_file(&file.path(), &target)?;
            moved += 1;
        }
        // `remove_dir` only removes empty folders, so anything left behind stays.
        let _ = fs::remove_dir(project.path());
    }
    let _ = fs::remove_dir(&old);
    Ok(moved)
}

/// Renames a file, or copies and deletes it when it's on another disk.
fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|e| format!("could not copy {}: {e}", from.display()))?;
    fs::remove_file(from).map_err(|e| format!("copied, but could not remove {}: {e}", from.display()))
}

/// Copies a project folder next to itself as "<name> (copy)" and renames the copy inside its
/// comilito.json, so both show up in the picker. Returns the new folder.
pub fn duplicate_project(dir: &Path) -> Result<PathBuf, String> {
    let mut project = storage::load_project(dir)?;
    let parent = dir.parent().ok_or("the project folder has no parent")?;
    let name = format!("{} (copy)", project.name);
    let target = storage::unique_path(parent, Path::new(&storage::safe_file_name(&name)));
    copy_dir(dir, &target, &mut 0, &mut |_, _| {}).inspect_err(|_| {
        let _ = fs::remove_dir_all(&target);
    })?;
    project.name = target.file_name().unwrap_or_default().to_string_lossy().into_owned();
    // Its own backups, not the original's (decision 0058).
    project.id = crate::project::new_id("prj");
    storage::save_project(&target, &project)?;
    Ok(target)
}

/// Renames when possible (instant, same disk). `rename` fails across disks, so then it copies
/// and deletes the original, removing a half-made copy if the copy fails.
/// `done` counts bytes moved so far, across calls; `progress` hears about each step.
fn move_dir(from: &Path, to: &Path, done: &mut u64, progress: &mut dyn FnMut(u64, &Path)) -> Result<(), String> {
    let size = dir_size(from);
    if fs::rename(from, to).is_ok() {
        *done += size;
        progress(*done, from);
        return Ok(());
    }
    let before = *done;
    if let Err(e) = copy_dir(from, to, done, progress) {
        *done = before;
        let _ = fs::remove_dir_all(to);
        return Err(e);
    }
    fs::remove_dir_all(from).map_err(|e| format!("copied, but could not remove {}: {e}", from.display()))
}

fn copy_dir(from: &Path, to: &Path, done: &mut u64, progress: &mut dyn FnMut(u64, &Path)) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("could not create {}: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("could not read {}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("could not read {}: {e}", from.display()))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &target, done, progress)?;
        } else {
            *done += fs::copy(&source, &target).map_err(|e| format!("could not copy {}: {e}", source.display()))?;
            progress(*done, &source);
        }
    }
    Ok(())
}

/// Bytes in all the files under `path` (unreadable parts count as 0; it's only for progress).
fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            _ => e.metadata().map(|m| m.len()).unwrap_or(0),
        })
        .sum()
}

/// Compares two folders, also when one of them is written differently (a trailing slash, a
/// symlink); `canonicalize` fails for folders that don't exist yet, which then can't be the same.
fn same_place(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectDefaults;
    use crate::storage::{create_project, test_support::TempDir};

    #[test]
    fn moves_projects_but_nothing_else() {
        let root = TempDir::new();
        let from = root.0.join("old");
        let to = root.0.join("new");
        create_project(&from, "One", ProjectDefaults::default()).unwrap();
        create_project(&from, "Two", ProjectDefaults::default()).unwrap();
        fs::write(from.join("shopping list.txt"), "eggs").unwrap();

        assert_eq!(move_projects(&from, &to).unwrap(), 2);
        assert!(to.join("One").join(PROJECT_FILE).is_file());
        assert!(to.join("Two").join(PROJECT_FILE).is_file());
        // The unrelated file stays, so the old folder does too.
        assert!(from.join("shopping list.txt").is_file());
        assert!(!to.join("shopping list.txt").exists());
    }

    #[test]
    fn migrates_old_backups_without_overwriting() {
        let root = TempDir::new();
        let projects = root.0.join("projects");
        let backups = root.0.join("data").join("backups");
        let old = projects.join(OLD_BACKUPS).join("Strahd");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("Strahd 2026-11-01 10-00.comilito"), "old").unwrap();
        fs::write(old.join("Strahd 2026-11-02 10-00.comilito"), "old").unwrap();
        fs::create_dir_all(backups.join("Strahd")).unwrap();
        fs::write(backups.join("Strahd").join("Strahd 2026-11-02 10-00.comilito"), "new").unwrap();

        assert_eq!(migrate_backups(&projects, &backups).unwrap(), 1);
        assert!(backups.join("Strahd").join("Strahd 2026-11-01 10-00.comilito").is_file());
        let kept = fs::read_to_string(backups.join("Strahd").join("Strahd 2026-11-02 10-00.comilito")).unwrap();
        assert_eq!(kept, "new", "existing backups are never overwritten");
        // The clashing file stays behind, so the old folder does too.
        assert!(old.join("Strahd 2026-11-02 10-00.comilito").is_file());
        assert_eq!(migrate_backups(&root.0.join("nowhere"), &backups).unwrap(), 0);
    }

    #[test]
    fn reports_progress_up_to_the_total() {
        let root = TempDir::new();
        let from = root.0.join("old");
        let dir = create_project(&from, "One", ProjectDefaults::default()).unwrap();
        fs::write(dir.join("audio").join("music").join("theme.ogg"), vec![0u8; 5000]).unwrap();

        let plan = plan_move(&from, &root.0.join("new")).unwrap();
        assert!(plan.total_bytes >= 5000);
        let mut last = 0;
        run_move(&plan, &mut |done, _| last = done).unwrap();
        assert_eq!(last, plan.total_bytes);
    }

    #[test]
    fn refuses_name_clashes_before_moving_anything() {
        let root = TempDir::new();
        let from = root.0.join("old");
        let to = root.0.join("new");
        create_project(&from, "One", ProjectDefaults::default()).unwrap();
        create_project(&from, "Two", ProjectDefaults::default()).unwrap();
        fs::create_dir_all(to.join("Two")).unwrap();

        assert!(move_projects(&from, &to).is_err());
        assert!(from.join("One").join(PROJECT_FILE).is_file());
        assert!(from.join("Two").join(PROJECT_FILE).is_file());
    }

    #[test]
    fn refuses_moving_into_a_project() {
        let root = TempDir::new();
        let dir = create_project(&root.0, "One", ProjectDefaults::default()).unwrap();
        assert!(move_projects(&root.0, &dir.join("inside")).is_err());
        assert!(move_projects(&root.0, &root.0).is_err());
    }

    #[test]
    fn duplicates_a_project_under_a_new_name() {
        let root = TempDir::new();
        let dir = create_project(&root.0, "Strahd", ProjectDefaults::default()).unwrap();
        fs::write(dir.join("audio").join("music").join("theme.ogg"), "x").unwrap();

        let copy = duplicate_project(&dir).unwrap();
        assert_eq!(copy, root.0.join("Strahd (copy)"));
        assert_eq!(storage::load_project(&copy).unwrap().name, "Strahd (copy)");
        assert!(copy.join("audio").join("music").join("theme.ogg").is_file());

        let again = duplicate_project(&dir).unwrap();
        assert_eq!(storage::load_project(&again).unwrap().name, "Strahd (copy) (2)");
    }
}
