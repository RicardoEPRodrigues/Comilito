//! Everything the running app knows, in one struct, shared by all commands behind one Mutex.
//!
//! One lock for everything keeps things simple: a command locks, changes what it needs,
//! re-applies the mix and unlocks. With separate locks (one for the project, one for the
//! engine) we'd have to worry about taking them in the same order everywhere to avoid
//! deadlocks, for no real gain in an app this size.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Child,
    sync::{Arc, Mutex, mpsc},
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
    /// Where backups go: `<app data folder>/backups` (decision 0026).
    pub backups_dir: PathBuf,
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
    /// Unfinished downloads from an earlier run, waiting for their project to be opened.
    pub waiting_downloads: Vec<crate::downloads::SavedDownload>,
    /// The yt-dlp process of the running download, so quitting can stop it.
    pub download_process: Option<Arc<Mutex<Child>>>,
    /// The app's data folder (logs, backups, `downloads.json`).
    pub data_dir: PathBuf,
    /// True while the ticker writes a backup (for the warning when quitting).
    pub backing_up: bool,
    /// Set once the app is really quitting (the user confirmed, or nothing was busy).
    pub quitting: bool,
    /// Videos being converted to WebM (video_convert.rs), how to wake that worker, and its
    /// running ffmpeg (stopped on quit).
    pub conversions: Vec<crate::video_convert::ConversionJob>,
    /// The GPU encoders that work on this computer (decision 0051), found the first time
    /// they're needed. `None` until then, and again after ffmpeg is replaced.
    pub video_encoders: Option<Vec<crate::video_encoders::Encoder>>,
    pub conversion_wakeup: Option<mpsc::Sender<()>>,
    pub conversion_process: Option<Arc<Mutex<Child>>>,
    /// Unfinished conversions waiting for their project to open (from the last run, or parked
    /// when their project closed).
    pub waiting_conversions: Vec<crate::video_convert::SavedConversion>,
    /// New videos to look at, and those to offer for conversion.
    pub video_offers: crate::video_convert::Offers,
    /// The video window (video_window.rs): whether it's open, the video effect it's playing,
    /// and where each video stopped (ms), kept while the app runs.
    pub video_window_open: bool,
    pub video_effect: Option<String>,
    pub video_positions: HashMap<String, u64>,
    /// When `video_positions` was last written to disk (see `video_window::save_video_position`).
    pub positions_saved: Instant,
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
    /// The Library's video folders, per kind (decisions 0035 and 0039).
    pub video_folders: crate::video::VideoFolders,
    /// New audio files seen in the folder but not added yet, with their size. A file is only
    /// added once its size is the same on two scans in a row, so we never pick up a file
    /// that's still being copied or synced.
    pub pending_files: HashMap<String, u64>,
    /// How long the last change took to fade the category volumes, so the mixer bar's
    /// sliders can glide along with the sound instead of jumping.
    pub levels_fade_ms: u32,
}

/// Ids of sounds and videos whose file is missing (both kinds share one list; their ids
/// never collide).
pub fn all_missing(dir: &Path, project: &Project) -> Vec<String> {
    let mut missing = storage::missing_sounds(dir, project);
    missing.extend(crate::video::missing(dir, project));
    missing
}

/// What `refresh_from_disk` changed.
#[derive(Debug, Default, PartialEq)]
pub struct DiskRefresh {
    pub added: usize,
    /// Ids of videos found in `video/` and added (they may need converting).
    pub new_videos: Vec<String>,
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

        // The same for videos in `video/` (decision 0035).
        let known: HashSet<String> = self.project.videos.iter().map(|v| v.file.path.clone()).collect();
        for (path, size, kind) in crate::video::scan_files(&self.dir).into_iter().filter(|(p, _, _)| !known.contains(p)) {
            if self.pending_files.get(&path) != Some(&size) {
                still_pending.insert(path, size);
                continue;
            }
            let Ok(info) = storage::file_info(&self.dir, Path::new(&path)) else {
                continue;
            };
            let name = Path::new(&path)
                .file_stem()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Video".into());
            let video = crate::video::new_video(name, info, kind);
            result.new_videos.push(video.id.clone());
            self.project.videos.push(video);
        }
        self.pending_files = still_pending;

        let missing = all_missing(&self.dir, &self.project);
        if missing != self.missing {
            self.missing = missing;
            result.missing_changed = true;
        }
        let folders = storage::list_folders(&self.dir);
        if folders != self.folders {
            self.folders = folders;
            result.missing_changed = true;
        }
        let video_folders = crate::video::list_folders(&self.dir);
        if video_folders != self.video_folders {
            self.video_folders = video_folders;
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
    pub video_folders: crate::video::VideoFolders,
    /// The video window (decision 0035): open or not, and the video effect it's playing.
    pub video_window_open: bool,
    pub video_effect: Option<String>,
    pub stack_status: StackStatus,
    /// Preset groups in display order (including empty ones).
    pub preset_groups: Vec<String>,
    /// How long the category volumes take to reach `mix.volumes` (see `OpenProject`).
    pub levels_fade_ms: u32,
}

impl AppState {
    pub fn projects_root(&self) -> PathBuf {
        self.prefs
            .projects_folder
            .clone()
            .unwrap_or_else(|| self.default_projects_root.clone())
    }

    /// `<app data folder>/backups` (decisions 0007 and 0026): with the app's own files, so the
    /// projects folder holds only projects, and never inside a project (so never in exports).
    pub fn backups_root(&self) -> PathBuf {
        self.backups_dir.clone()
    }

    /// What quitting now would interrupt, in words for the warning. Empty when nothing is.
    pub fn busy_work(&self) -> Vec<String> {
        let plural = |n: usize, one: &str, many: &str| if n == 1 { format!("1 {one}") } else { format!("{n} {many}") };
        let mut work = Vec::new();
        if self.moving_projects {
            work.push("Moving the projects folder. Stopping now leaves projects in both places.".to_string());
        }
        let downloads = self
            .downloads
            .iter()
            .filter(|j| matches!(j.status, crate::downloads::JobStatus::Queued | crate::downloads::JobStatus::Running))
            .count();
        if downloads > 0 {
            work.push(format!(
                "{} unfinished. They continue the next time you open the project.",
                plural(downloads, "download", "downloads")
            ));
        }
        if self.backing_up {
            work.push("Writing a backup. It would be left incomplete.".to_string());
        }
        let converting = self
            .conversions
            .iter()
            .filter(|j| matches!(j.status, crate::downloads::JobStatus::Queued | crate::downloads::JobStatus::Running))
            .count();
        if converting > 0 {
            work.push(format!(
                "Converting {}. They start again the next time you open the project.",
                plural(converting, "video", "videos")
            ));
        }
        if let Some(open) = &self.open {
            let hashing = open
                .project
                .files()
                .filter(|(id, file)| file.sha256.is_none() && !open.missing.iter().any(|m| m == id))
                .count();
            if hashing > 0 {
                work.push(format!(
                    "Fingerprinting {} (used to find moved or missing files). It carries on next time.",
                    plural(hashing, "file", "files")
                ));
            }
        }
        work
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
        crate::flatpak::check_writable_folder(&dir)?;
        let mut project = storage::load_project(&dir)?;
        self.close_project()?;
        crate::video_convert::clean_leftovers(&dir);
        // Videos from before the two kinds move into video/ambience/ (decision 0039), once.
        let moved_videos = crate::video::move_into_kind_folders(&dir, &mut project)
            .inspect_err(|e| crate::logging::warn(&format!("could not sort the videos into their folders: {e}")))
            .unwrap_or(false);

        // A project from before ids (decision 0058) gets one now, saved at once: its backups are
        // filed by it, so it must be the same the next time the project opens.
        if project.id.is_empty() {
            project.id = crate::project::new_id("prj");
            if let Err(e) = storage::save_project(&dir, &project) {
                crate::logging::warn(&format!("could not save the project's new id: {e}"));
            }
        }

        let missing = all_missing(&dir, &project);
        let mix = target_mix(&project);
        self.engine.apply_levels(&project, &mix, Duration::ZERO);
        let folders = storage::list_folders(&dir);
        self.open = Some(OpenProject {
            folders,
            video_folders: crate::video::list_folders(&dir),
            known_file_time: storage::project_file_modified(&dir),
            dir,
            project,
            mix,
            missing,
            // Save soon if the videos were moved, so the paths on disk and in the file agree.
            dirty_since: moved_videos.then(Instant::now),
            changed_on_disk: false,
            changed_since_backup: false,
            last_backup: Instant::now(),
            analysis_failed: Vec::new(),
            pending_files: HashMap::new(),
            levels_fade_ms: 0,
        });
        // The stack is restored but not started: the DM presses Space when ready.
        if let Some(open) = &self.open {
            let dir = open.dir.clone();
            crate::downloads::resume_for(self, &dir);
            crate::video_convert::resume_for(self, &dir);
            // Where its videos left off last time (resumed by the video window).
            self.video_positions = crate::video_window::load_positions(&self.data_dir, &dir);
            self.prefs.note_opened(&dir);
            // Opened from elsewhere (decision 0042): remember it so the home screen lists it.
            if !storage::is_in_root(&dir, &self.projects_root()) {
                self.prefs.note_elsewhere(&dir);
            }
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
        if let Some(open) = &self.open {
            crate::video_window::store_positions(&self.data_dir, &open.dir, &open.project, &self.video_positions);
            let dir = open.dir.clone();
            crate::video_convert::park_for(self, &dir);
        }
        self.engine.stop_all(Duration::from_millis(200));
        self.engine.clear_cache();
        self.open = None;
        self.video_effect = None;
        self.video_positions.clear();
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
        // The category volumes follow the top scene, so a stack change can change them too;
        // they fade along with the sounds.
        let levels_fade = transition.max(VOLUME_SMOOTHING);
        open.levels_fade_ms = levels_fade.as_millis() as u32;
        self.engine.apply_levels(&open.project, &open.mix, levels_fade);
        self.engine
            .apply_mix(&open.project, &open.dir, &open.mix, transition)
    }

    /// Plays through another output (`None`: the system default). kira can't move a running
    /// mix to another device, so the engine is rebuilt: what was playing starts again from
    /// the beginning on the new output, and a paused mix stays stopped. Returns problems to
    /// show the user.
    pub fn switch_output(&mut self, device: Option<&str>) -> Vec<String> {
        let (mut engine, warning) = match AudioEngine::new(device) {
            Ok(created) => created,
            Err(e) => return vec![format!("Couldn't switch the audio output: {e}")],
        };
        let was_playing = self.engine.playback_transport() == crate::audio::Transport::Playing;
        let mut problems: Vec<String> = warning.into_iter().collect();
        if let Some(open) = &self.open {
            engine.apply_levels(&open.project, &open.mix, Duration::ZERO);
            if was_playing {
                let fade = Duration::from_millis(self.prefs.playback.pause_fade_ms as u64);
                problems.extend(engine.toggle_play(&open.project, &open.dir, &open.mix, fade));
            }
        }
        // The old engine is dropped here, which closes its output.
        self.engine = engine;
        problems
    }

    /// Re-applies volumes after a small edit (a slider), with only a short smoothing fade.
    pub fn levels_changed(&mut self) -> Vec<String> {
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
                    .files()
                    .filter(|(id, file)| file.sha256.is_none() && !open.missing.iter().any(|m| m == id))
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
                video_folders: open.video_folders.clone(),
                video_window_open: self.video_window_open,
                video_effect: self.video_effect.clone(),
                stack_status: open.project.stack_status(),
                preset_groups: open.project.all_preset_groups(),
                levels_fade_ms: open.levels_fade_ms,
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
                video_folders: Default::default(),
                video_window_open: self.video_window_open,
                video_effect: None,
                stack_status: StackStatus::Empty,
                preset_groups: Vec::new(),
                levels_fade_ms: 0,
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
    if let Some(open) = &state.open {
        crate::video::allow_media(app, &open.dir);
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
            missing: all_missing(&dir, &project),
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
            video_folders: Default::default(),
            levels_fade_ms: 0,
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
    fn videos_in_the_video_folder_are_added_and_missing_ones_noticed() {
        let root = TempDir::new();
        let dir = storage::create_project(&root.0, "P", ProjectDefaults::default()).unwrap();
        let mut open = open_project(dir.clone());
        std::fs::create_dir_all(dir.join("video/ambience/Castle")).unwrap();
        let file = dir.join("video/ambience/Castle/gate.webm");
        std::fs::write(&file, b"video").unwrap();

        assert!(open.refresh_from_disk().new_videos.is_empty(), "first sighting waits");
        let found = open.refresh_from_disk();
        assert_eq!(found.new_videos.len(), 1);
        assert_eq!(found.added, 0, "videos aren't counted as sounds");
        assert_eq!(open.project.videos[0].name, "gate");
        assert_eq!(open.video_folders.ambience, ["Castle"]);
        assert_eq!(open.project.videos[0].kind, crate::project::VideoKind::Ambience);

        std::fs::remove_file(&file).unwrap();
        open.refresh_from_disk();
        assert_eq!(open.missing, [open.project.videos[0].id.clone()]);
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
