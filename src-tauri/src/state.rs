//! Everything the running app knows, in one struct, shared by all commands behind one Mutex.
//!
//! One lock for everything keeps things simple: a command locks, changes what it needs,
//! re-applies the mix and unlocks. With separate locks (one for the project, one for the
//! engine) we'd have to worry about taking them in the same order everywhere to avoid
//! deadlocks, for no real gain in an app this size.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::{Duration, Instant, SystemTime},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    audio::{AudioEngine, PlaybackInfo, VOLUME_SMOOTHING},
    bundle,
    mix::{TargetMix, target_mix},
    prefs::{self, Preferences},
    project::{PerCategory, Project, Sound, StackStatus},
    storage,
};

/// How long after the last change the project is written to disk. Dragging a slider
/// changes the project many times a second; this turns that into one write.
pub const SAVE_DELAY: Duration = Duration::from_secs(1);

pub struct AppState {
    pub engine: AudioEngine,
    pub prefs: Preferences,
    pub config_dir: PathBuf,
    /// `~/Documents/Comilito`, used unless the preferences say otherwise.
    pub default_projects_root: PathBuf,
    /// True while `move_projects_folder` moves projects on its background thread. Opening,
    /// creating, duplicating or deleting projects is refused meanwhile.
    pub moving_projects: bool,
    /// The bundled sample audio, if it was found at startup.
    pub sample_dir: Option<PathBuf>,
    pub open: Option<OpenProject>,
    /// Where yt-dlp and ffmpeg are installed (tools.rs).
    pub tools_dir: PathBuf,
    /// The download queue (downloads.rs), and how to wake its worker thread.
    pub downloads: Vec<crate::downloads::DownloadJob>,
    pub download_wakeup: Option<mpsc::Sender<()>>,
    /// Waveform peaks of recently viewed files, keyed by path and modification time.
    pub waveforms: Vec<(String, Arc<crate::waveform::Peaks>)>,
}

pub struct OpenProject {
    pub dir: PathBuf,
    pub project: Project,
    /// The mix computed from the current stack (recomputed on every change).
    pub mix: TargetMix,
    /// Ids of sounds whose file is missing.
    pub missing: Vec<String>,
    /// Set when the project changed and hasn't been saved yet.
    pub dirty_since: Option<Instant>,
    /// Modification time of comilito.json after our last save or load, to notice when
    /// another program (a sync tool) replaces it.
    pub known_file_time: Option<SystemTime>,
    pub changed_on_disk: bool,
    /// For backups: whether anything changed since the last one, and when it was made.
    pub changed_since_backup: bool,
    pub last_backup: Instant,
    /// Sounds whose loudness couldn't be measured (undecodable file), so we don't retry.
    pub analysis_failed: Vec<String>,
    /// The Library folders on disk (subfolders of `audio/<category>/`).
    pub folders: PerCategory<Vec<String>>,
    /// New audio files seen in the folder but not added yet, with their size. A file is only
    /// added once its size is the same on two scans in a row, so we never pick up a file
    /// that's still being copied or synced.
    pub pending_files: HashMap<String, u64>,
}

/// What `refresh_from_disk` changed.
#[derive(Debug, Default, PartialEq)]
pub struct DiskRefresh {
    pub added: usize,
    pub missing_changed: bool,
}

impl OpenProject {
    /// Compares the project's `audio/` folders with the library: audio files that appeared
    /// become new sounds (category from their folder), and the list of sounds with a missing
    /// file is brought up to date. A removed file only marks its sound as missing; it isn't
    /// deleted, since a sync tool may just be replacing it.
    pub fn refresh_from_disk(&mut self) -> DiskRefresh {
        let found = storage::scan_audio_files(&self.dir);
        let known: HashSet<String> = self.project.sounds.iter().map(|s| s.file.path.clone()).collect();

        let mut result = DiskRefresh::default();
        let mut still_pending = HashMap::new();
        for file in found.into_iter().filter(|f| !known.contains(&f.path)) {
            if self.pending_files.get(&file.path) != Some(&file.size) {
                // First sighting, or still growing: look again next time.
                still_pending.insert(file.path, file.size);
                continue;
            }
            let Ok(info) = storage::file_info(&self.dir, Path::new(&file.path)) else {
                continue;
            };
            let name = Path::new(&file.path)
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Sound".into());
            self.project.sounds.push(Sound::new(file.category, name, info));
            result.added += 1;
        }
        self.pending_files = still_pending;

        let missing = storage::missing_sounds(&self.dir, &self.project);
        if missing != self.missing {
            self.missing = missing;
            result.missing_changed = true;
        }
        let folders = storage::list_folders(&self.dir);
        if folders != self.folders {
            self.folders = folders;
            result.missing_changed = true;
        }
        result
    }

    /// The next sound that still needs its loudness measured, if any.
    pub fn next_to_analyze(&self) -> Option<&crate::project::Sound> {
        self.project.sounds.iter().find(|snd| {
            snd.file.needs_analysis()
                && !self.missing.contains(&snd.id)
                && !self.analysis_failed.contains(&snd.id)
        })
    }
}

/// What the UI renders. Sent whole with every `state-changed` event, so the UI never has
/// to work out what changed: it just shows the latest snapshot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub project: Option<Project>,
    pub project_dir: Option<PathBuf>,
    pub missing: Vec<String>,
    pub mix: TargetMix,
    pub playback: PlaybackInfo,
    pub changed_on_disk: bool,
    /// How many sounds still wait for their sha256 (computed in the background).
    pub hashing_remaining: usize,
    /// How many sounds still wait for their loudness to be measured.
    pub analyzing_remaining: usize,
    pub folders: PerCategory<Vec<String>>,
    pub stack_status: StackStatus,
    /// Preset groups in display order (including empty ones).
    pub preset_groups: Vec<String>,
}

impl AppState {
    pub fn projects_root(&self) -> PathBuf {
        self.prefs
            .projects_folder
            .clone()
            .unwrap_or_else(|| self.default_projects_root.clone())
    }

    /// `<projects folder>/Backups` (decision 0007). Outside every project folder, so backups
    /// never end up inside exports.
    pub fn backups_root(&self) -> PathBuf {
        self.projects_root().join(crate::relocate::BACKUPS)
    }

    /// The open project, or an error for commands that need one.
    pub fn open_mut(&mut self) -> Result<&mut OpenProject, String> {
        self.open.as_mut().ok_or_else(|| "no project is open".to_string())
    }

    pub fn open_ref(&self) -> Result<&OpenProject, String> {
        self.open.as_ref().ok_or_else(|| "no project is open".to_string())
    }

    pub fn open_project(&mut self, dir: PathBuf) -> Result<(), String> {
        self.not_moving()?;
        let project = storage::load_project(&dir)?;
        self.close_project()?;

        self.engine.apply_levels(&project, Duration::ZERO);
        let missing = storage::missing_sounds(&dir, &project);
        let mix = target_mix(&project);
        let folders = storage::list_folders(&dir);
        self.open = Some(OpenProject {
            folders,
            known_file_time: storage::project_file_modified(&dir),
            dir,
            project,
            mix,
            missing,
            dirty_since: None,
            changed_on_disk: false,
            changed_since_backup: false,
            last_backup: Instant::now(),
            analysis_failed: Vec::new(),
            pending_files: HashMap::new(),
        });
        // The stack is restored but not started: the DM presses Space when ready.
        if let Some(open) = &self.open {
            let dir = open.dir.clone();
            self.prefs.note_opened(&dir);
            // Not being able to remember it isn't worth failing the open for.
            if let Err(e) = self.save_prefs() {
                crate::logging::warn(&format!("could not save the recent projects: {e}"));
            }
        }
        Ok(())
    }

    /// Saves (if needed), backs up (if anything changed) and closes the open project,
    /// stopping all its sound.
    pub fn close_project(&mut self) -> Result<(), String> {
        self.save_if_dirty()?;
        let backups_dir = self.backups_root();
        if let Some(open) = &self.open
            && open.changed_since_backup
            && let Err(e) = bundle::write_backup(&open.dir, &open.project, &backups_dir)
        {
            eprintln!("backup on close failed: {e}");
        }
        self.engine.stop_all(Duration::from_millis(200));
        self.engine.clear_cache();
        self.open = None;
        Ok(())
    }

    pub fn save_if_dirty(&mut self) -> Result<(), String> {
        if let Some(open) = &mut self.open
            && open.dirty_since.is_some()
        {
            storage::save_project(&open.dir, &open.project)?;
            open.dirty_since = None;
            open.known_file_time = storage::project_file_modified(&open.dir);
        }
        Ok(())
    }

    /// Call after changing the project: marks it for saving, recomputes the target mix and
    /// fades the engine to it over `transition`. Returns problems to show the user.
    pub fn project_changed(&mut self, transition: Duration) -> Vec<String> {
        // `self.open` and `self.engine` are different fields, so Rust lets us borrow one
        // mutably (`open`) while also using the other (`self.engine`). Through a method
        // call like `self.open_mut()` that wouldn't work: the method borrows all of `self`.
        let Some(open) = &mut self.open else {
            return Vec::new();
        };
        open.dirty_since.get_or_insert_with(Instant::now);
        open.changed_since_backup = true;
        open.mix = target_mix(&open.project);
        self.engine
            .apply_mix(&open.project, &open.dir, &open.mix, transition)
    }

    /// Re-applies volumes after a small edit (a slider), with only a short smoothing fade.
    pub fn levels_changed(&mut self) -> Vec<String> {
        if let Some(open) = &self.open {
            self.engine.apply_levels(&open.project, VOLUME_SMOOTHING);
        }
        self.project_changed(VOLUME_SMOOTHING)
    }

    pub fn snapshot(&self) -> Snapshot {
        match &self.open {
            Some(open) => Snapshot {
                project: Some(open.project.clone()),
                project_dir: Some(open.dir.clone()),
                missing: open.missing.clone(),
                mix: open.mix.clone(),
                playback: self.engine.playback_info(&open.mix),
                changed_on_disk: open.changed_on_disk,
                hashing_remaining: open
                    .project
                    .sounds
                    .iter()
                    .filter(|snd| snd.file.sha256.is_none() && !open.missing.contains(&snd.id))
                    .count(),
                analyzing_remaining: open
                    .project
                    .sounds
                    .iter()
                    .filter(|snd| {
                        snd.file.needs_analysis()
                            && !open.missing.contains(&snd.id)
                            && !open.analysis_failed.contains(&snd.id)
                    })
                    .count(),
                folders: open.folders.clone(),
                stack_status: open.project.stack_status(),
                preset_groups: open.project.all_preset_groups(),
            },
            None => Snapshot {
                project: None,
                project_dir: None,
                missing: Vec::new(),
                mix: TargetMix::default(),
                playback: self.engine.playback_info(&TargetMix::default()),
                changed_on_disk: false,
                hashing_remaining: 0,
                analyzing_remaining: 0,
                folders: PerCategory::default(),
                stack_status: StackStatus::Empty,
                preset_groups: Vec::new(),
            },
        }
    }

    /// An error while the projects are being moved (see `moving_projects`).
    pub fn not_moving(&self) -> Result<(), String> {
        if self.moving_projects {
            return Err("the projects are being moved; wait until that's done".into());
        }
        Ok(())
    }

    pub fn save_prefs(&self) -> Result<(), String> {
        prefs::save(&self.config_dir, &self.prefs)
    }
}

/// Sends the latest snapshot to the UI, plus any problems as `notice` events.
/// The `AppHandle` is Tauri's handle to the running app; it can be cloned and used from
/// any thread to emit events.
pub fn emit_state(app: &AppHandle, state: &AppState, problems: Vec<String>) {
    for message in problems {
        crate::logging::warn(&message);
        let _ = app.emit("notice", message);
    }
    // `let _ =` explicitly ignores the Result: if the window is gone there's no one to tell.
    let _ = app.emit("state-changed", state.snapshot());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project::ProjectDefaults, storage::test_support::TempDir};

    fn open_project(dir: PathBuf) -> OpenProject {
        let project = storage::load_project(&dir).unwrap();
        OpenProject {
            mix: target_mix(&project),
            missing: storage::missing_sounds(&dir, &project),
            known_file_time: None,
            dir,
            project,
            dirty_since: None,
            changed_on_disk: false,
            changed_since_backup: false,
            last_backup: Instant::now(),
            analysis_failed: Vec::new(),
            pending_files: HashMap::new(),
            folders: PerCategory::default(),
        }
    }

    #[test]
    fn new_files_are_added_once_their_size_is_stable() {
        let root = TempDir::new();
        let dir = storage::create_project(&root.0, "P", ProjectDefaults::default()).unwrap();
        let mut open = open_project(dir.clone());
        let file = dir.join("audio/ambience/rain.ogg");

        std::fs::write(&file, b"half").unwrap();
        assert_eq!(open.refresh_from_disk().added, 0, "first sighting waits");
        std::fs::write(&file, b"half copied").unwrap();
        assert_eq!(open.refresh_from_disk().added, 0, "size changed, still waiting");
        assert_eq!(open.refresh_from_disk().added, 1, "stable now");

        let sound = &open.project.sounds[0];
        assert_eq!(sound.name, "rain");
        assert_eq!(sound.category, crate::project::Category::Ambience);
        assert_eq!(open.refresh_from_disk().added, 0, "not added twice");
    }

    #[test]
    fn removed_files_mark_sounds_missing_and_returning_files_unmark_them() {
        let root = TempDir::new();
        let dir = storage::create_project(&root.0, "P", ProjectDefaults::default()).unwrap();
        let file = dir.join("audio/music/theme.mp3");
        std::fs::write(&file, b"music").unwrap();
        let mut open = open_project(dir.clone());
        open.refresh_from_disk();
        open.refresh_from_disk();
        assert_eq!(open.project.sounds.len(), 1);

        std::fs::remove_file(&file).unwrap();
        let r = open.refresh_from_disk();
        assert!(r.missing_changed);
        assert_eq!(open.missing.len(), 1);
        assert_eq!(open.project.sounds.len(), 1, "the sound is kept, only marked missing");

        std::fs::write(&file, b"music").unwrap();
        assert!(open.refresh_from_disk().missing_changed);
        assert!(open.missing.is_empty());
    }
}
