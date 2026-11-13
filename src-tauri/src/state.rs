//! Everything the running app knows, in one struct, shared by all commands behind one Mutex.
//!
//! One lock for everything keeps things simple: a command locks, changes what it needs,
//! re-applies the mix and unlocks. With separate locks (one for the project, one for the
//! engine) we'd have to worry about taking them in the same order everywhere to avoid
//! deadlocks, for no real gain in an app this size.

use std::{
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    audio::{AudioEngine, PlaybackInfo, VOLUME_SMOOTHING},
    bundle,
    mix::{TargetMix, target_mix},
    prefs::{self, Preferences},
    project::Project,
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
    pub open: Option<OpenProject>,
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
        self.projects_root().join("Backups")
    }

    /// The open project, or an error for commands that need one.
    pub fn open_mut(&mut self) -> Result<&mut OpenProject, String> {
        self.open.as_mut().ok_or_else(|| "no project is open".to_string())
    }

    pub fn open_ref(&self) -> Result<&OpenProject, String> {
        self.open.as_ref().ok_or_else(|| "no project is open".to_string())
    }

    pub fn open_project(&mut self, dir: PathBuf) -> Result<(), String> {
        let project = storage::load_project(&dir)?;
        self.close_project()?;

        self.engine.apply_levels(&project, Duration::ZERO);
        let missing = storage::missing_sounds(&dir, &project);
        let mix = target_mix(&project);
        self.open = Some(OpenProject {
            known_file_time: storage::project_file_modified(&dir),
            dir,
            project,
            mix,
            missing,
            dirty_since: None,
            changed_on_disk: false,
            changed_since_backup: false,
            last_backup: Instant::now(),
        });
        // The stack is restored but not started: the DM presses Space when ready.
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
            },
            None => Snapshot {
                project: None,
                project_dir: None,
                missing: Vec::new(),
                mix: TargetMix::default(),
                playback: self.engine.playback_info(&TargetMix::default()),
                changed_on_disk: false,
                hashing_remaining: 0,
            },
        }
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
        let _ = app.emit("notice", message);
    }
    // `let _ =` explicitly ignores the Result: if the window is gone there's no one to tell.
    let _ = app.emit("state-changed", state.snapshot());
}
