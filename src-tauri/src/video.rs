//! Videos (decisions 0035 and 0039): the files in the project's `video/ambience/` and
//! `video/effects/` folders and the Library commands for them. Kept in one module, apart from the audio code, so the feature stays easy to find
//! (and could one day move into a plugin). Videos are never played here: the video window
//! (a webview) plays them, muted.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::{
    commands::{AppStateRef, lock},
    project::{FileInfo, Project, Shelf, Video, VideoKind},
    state::{OpenProject, emit_state},
    storage,
};

/// Lets the webviews load files from the open project's folder through the asset protocol
/// (`asset://` URLs), so the video window and the Library can play its videos. Nothing else
/// on disk is reachable that way: the static scope in tauri.conf.json is empty.
/// Called with every state update; only the first call for a folder does anything.
pub fn allow_media(app: &AppHandle, dir: &Path) {
    // A `static` holds one value for the whole program (like a C global). It's wrapped in a
    // Mutex because any thread may call this.
    static ALLOWED: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());
    let Ok(mut allowed) = ALLOWED.lock() else { return };
    if allowed.iter().any(|d| d == dir) {
        return;
    }
    match app.asset_protocol_scope().allow_directory(dir, true) {
        Ok(()) => allowed.push(dir.to_path_buf()),
        Err(e) => crate::logging::warn(&format!("could not allow the video files of {}: {e}", dir.display())),
    }
}

/// File extensions treated as video. WebM plays everywhere; the others may need converting
/// (see `VideoInfo::plays_everywhere`).
pub const VIDEO_EXTENSIONS: [&str; 7] = ["webm", "mp4", "m4v", "mkv", "mov", "avi", "ogv"];

pub fn is_video_path(path: &Path) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| VIDEO_EXTENSIONS.contains(&e.as_str()))
}

/// The Library's video folders, per kind: subfolders of `video/ambience/` and `video/effects/`,
/// like "Ravenloft/Castle", sorted.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct VideoFolders {
    pub ambience: Vec<String>,
    pub effect: Vec<String>,
}

pub fn list_folders(dir: &Path) -> VideoFolders {
    VideoFolders {
        ambience: storage::subfolders(&storage::shelf_root(dir, Shelf::AmbienceVideo)),
        effect: storage::subfolders(&storage::shelf_root(dir, Shelf::VideoEffect)),
    }
}

/// Every video file under `video/ambience/` and `video/effects/`, as (path relative to the
/// project, size, the kind its folder says).
pub fn scan_files(dir: &Path) -> Vec<(String, u64, VideoKind)> {
    [VideoKind::Ambience, VideoKind::Effect]
        .into_iter()
        .flat_map(|kind| {
            storage::files_on_shelf(dir, kind.shelf(), &VIDEO_EXTENSIONS)
                .into_iter()
                .map(move |(path, size)| (path, size, kind))
        })
        .collect()
}

/// Ids of videos whose file is missing from the project folder.
pub fn missing(dir: &Path, project: &Project) -> Vec<String> {
    project
        .videos
        .iter()
        .filter(|v| !storage::absolute_audio_path(dir, &v.file).is_file())
        .map(|v| v.id.clone())
        .collect()
}

/// A new library video, not measured yet.
pub fn new_video(name: String, file: FileInfo, kind: VideoKind) -> Video {
    Video {
        id: crate::project::new_id("vid"),
        name,
        kind,
        file,
        info: None,
        tags: Vec::new(),
        favourite: false,
        source: None,
        last_used: None,
    }
}

/// The Library folder a video's file is in: the path between `video/<kind>/` and the file name.
pub fn folder_of(video: &Video) -> String {
    let root = format!("{}/", video.kind.shelf().root());
    let rest = video.file.path.strip_prefix(&root).unwrap_or(&video.file.path);
    rest.rsplit_once('/').map(|(folder, _)| folder.to_string()).unwrap_or_default()
}

fn name_from(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Video".into())
}

/// Projects from before the two kinds (decision 0039) kept videos straight in `video/`. Moves
/// everything there that isn't already in `video/ambience/` or `video/effects/` into
/// `video/ambience/` (keeping its folders), and points the videos at their new paths. Their
/// kind follows the folder they end up in. Returns whether anything changed (so the project
/// is saved).
pub fn move_into_kind_folders(dir: &Path, project: &mut Project) -> Result<bool, String> {
    let video_dir = dir.join("video");
    let ambience = storage::shelf_root(dir, Shelf::AmbienceVideo);
    let mut moved_any = false;
    if let Ok(entries) = std::fs::read_dir(&video_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            // The two kind folders stay; hidden files (a conversion's working file) too.
            if name == "ambience" || name == "effects" || name.starts_with('.') {
                continue;
            }
            std::fs::create_dir_all(&ambience).map_err(|e| format!("could not create {}: {e}", ambience.display()))?;
            let target = storage::unique_path(&ambience, Path::new(&name));
            std::fs::rename(entry.path(), &target).map_err(|e| format!("could not move {}: {e}", entry.path().display()))?;
            let old = format!("video/{name}");
            let new = storage::to_json_path(target.strip_prefix(dir).unwrap_or(&target));
            for video in &mut project.videos {
                if video.file.path == old {
                    video.file.path = new.clone();
                } else if let Some(rest) = video.file.path.strip_prefix(&format!("{old}/")) {
                    video.file.path = format!("{new}/{rest}");
                }
            }
            moved_any = true;
        }
    }
    // Every video's kind follows its folder, whatever the file said (and whether its file is
    // there or not: a missing one keeps its path).
    let mut changed = moved_any;
    for video in &mut project.videos {
        if let Some(rest) = video.file.path.strip_prefix("video/")
            && !rest.starts_with("ambience/")
            && !rest.starts_with("effects/")
        {
            video.file.path = format!("video/ambience/{rest}");
            changed = true;
        }
        let kind = if video.file.path.starts_with("video/effects/") { VideoKind::Effect } else { VideoKind::Ambience };
        if video.kind != kind {
            video.kind = kind;
            changed = true;
        }
    }
    Ok(changed)
}

// ---- Commands -------------------------------------------------------------------------------

/// Whether the webviews must stream videos into a MediaSource themselves (decision 0037):
/// true on Linux, where WebKitGTK won't play `<video src="asset://…">` (and stalls on
/// `http://` media too). Elsewhere a plain `src` works.
#[tauri::command]
pub fn video_streaming() -> bool {
    cfg!(target_os = "linux")
}

/// Copies video files into `video/<kind>/<folder>/` and adds them to the library. Returns
/// their ids.
#[tauri::command]
pub fn add_videos(
    kind: VideoKind,
    folder: Option<String>,
    paths: Vec<PathBuf>,
    app: AppHandle,
    state: AppStateRef,
) -> Result<Vec<String>, String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let folder = folder.unwrap_or_default();
    let (mut added, mut problems) = (Vec::new(), Vec::new());
    for path in paths {
        if !is_video_path(&path) {
            problems.push(format!("\"{}\" isn't a video Comilito can use", path.display()));
            continue;
        }
        match storage::copy_into_shelf(&open.dir, kind.shelf(), &folder, &path) {
            Ok(file) => {
                let video = new_video(name_from(&path), file, kind);
                added.push(video.id.clone());
                open.project.videos.push(video);
            }
            Err(e) => problems.push(e),
        }
    }
    open.video_folders = list_folders(&open.dir);
    // Looked at in the background; offered for conversion if they may not play.
    crate::video_convert::note_new(&mut s, &added, crate::video_convert::Origin::Added);
    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(added)
}

/// Saves edits made in the Library: name, tags and favourite.
#[tauri::command]
pub fn update_videos(videos: Vec<Video>, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    for edited in videos {
        let Some(video) = open.project.video_mut(&edited.id) else { continue };
        let name = edited.name.trim();
        if !name.is_empty() {
            video.name = name.to_string();
        }
        video.favourite = edited.favourite;
        let mut tags: Vec<String> = edited
            .tags
            .iter()
            .map(|t| t.trim().trim_start_matches('#').to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        tags.dedup();
        video.tags = tags;
    }
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Removes a video from the library and every scene, and moves its file to the trash (or
/// deletes it for good).
#[tauri::command]
pub fn remove_video(id: String, permanently: bool, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let index = open
        .project
        .videos
        .iter()
        .position(|v| v.id == id)
        .ok_or("that video no longer exists")?;
    let video = open.project.videos.remove(index);
    open.project.remove_dangling_references();
    open.missing.retain(|m| m != &id);

    let mut problems = Vec::new();
    let path = storage::absolute_audio_path(&open.dir, &video.file);
    let shared = open.project.videos.iter().any(|o| o.file.path == video.file.path);
    if path.exists() && !shared {
        let result = if permanently {
            std::fs::remove_file(&path).map_err(|e| e.to_string())
        } else {
            trash::delete(&path).map_err(|e| e.to_string())
        };
        if let Err(e) = result {
            problems.push(format!("could not remove {}: {e}", path.display()));
        }
    }
    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(())
}

/// Moves videos to a kind and Library folder: their files move to `video/<kind>/<folder>/`. A
/// video that changes kind leaves the scene slots that don't take it (an ambience video
/// turned into an effect leaves the playlists it was in), as for sounds changing category.
#[tauri::command]
pub fn move_videos(ids: Vec<String>, kind: VideoKind, folder: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut problems = Vec::new();
    for id in ids {
        let Some(video) = open.project.video(&id) else { continue };
        if (video.kind == kind && folder_of(video) == folder) || open.missing.contains(&id) {
            continue;
        }
        let changed_kind = video.kind != kind;
        match storage::move_into_shelf(&open.dir, &video.file, kind.shelf(), &folder) {
            Ok(file) => {
                if let Some(video) = open.project.video_mut(&id) {
                    video.file = file;
                    video.kind = kind;
                }
                if changed_kind {
                    for scene in &mut open.project.scenes {
                        match kind {
                            VideoKind::Effect => {
                                if let Some(playlist) = &mut scene.video {
                                    playlist.videos.retain(|v| v != &id);
                                }
                                if scene.video.as_ref().is_some_and(|p| p.videos.is_empty()) {
                                    scene.video = None;
                                }
                            }
                            VideoKind::Ambience => scene.video_effects.retain(|v| v != &id),
                        }
                    }
                }
            }
            Err(e) => problems.push(e),
        }
    }
    open.video_folders = list_folders(&open.dir);
    problems.extend(s.project_changed(Duration::ZERO));
    emit_state(&app, &s, problems);
    Ok(())
}

/// Shows a video's file selected in the system file manager.
#[tauri::command]
pub fn reveal_video(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let s = lock(&state)?;
    let open = s.open_ref()?;
    let video = open.project.video(&id).ok_or("that video no longer exists")?;
    let path = storage::absolute_audio_path(&open.dir, &video.file);
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| format!("could not open the file manager: {e}"))
}

/// Points a video (typically a missing one) at a file the user picked, copied into its folder.
#[tauri::command]
pub fn relink_video(id: String, path: PathBuf, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    relink(s.open_mut()?, &id, &path)?;
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Adds a video dragged onto a scene (or a tapestry entry) where its kind goes: an ambience
/// video to the playlist, a video effect to the video effects.
#[tauri::command]
pub fn add_video_to_scene(scene_id: String, video_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let kind = open.project.video(&video_id).ok_or("that video no longer exists")?.kind;
    let scene = open
        .project
        .scenes
        .iter_mut()
        .find(|sc| sc.id == scene_id)
        .ok_or("that scene no longer exists")?;
    match kind {
        VideoKind::Ambience => {
            let playlist = scene.video.get_or_insert_with(Default::default);
            if !playlist.videos.contains(&video_id) {
                playlist.videos.push(video_id);
            }
            // A scene with videos of its own has nothing to hide.
            scene.hides_video = false;
        }
        VideoKind::Effect => {
            if !scene.video_effects.contains(&video_id) {
                scene.video_effects.push(video_id);
            }
        }
    }
    let problems = s.project_changed(Duration::ZERO);
    emit_state(&app, &s, problems);
    Ok(())
}

/// Copies `source` into the project for video `id` (into the folder it was in) and points the
/// video at it.
pub fn relink(open: &mut OpenProject, id: &str, source: &Path) -> Result<(), String> {
    let video = open.project.video(id).ok_or("that video no longer exists")?;
    if !is_video_path(source) {
        return Err(format!("\"{}\" isn't a video Comilito can use", source.display()));
    }
    let folder = folder_of(video);
    let file = storage::copy_into_shelf(&open.dir, video.kind.shelf(), &folder, source)?;
    if let Some(video) = open.project.video_mut(id) {
        video.file = file;
        // A different file: measure it again.
        video.info = None;
    }
    open.missing.retain(|m| m != id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    fn file(path: &str) -> FileInfo {
        FileInfo {
            path: path.into(),
            size: 0,
            modified: String::new(),
            sha256: None,
            loudness: None,
        }
    }

    #[test]
    fn finds_video_files_and_folders_per_kind() {
        let dir = TempDir::new();
        std::fs::create_dir_all(dir.0.join("video/ambience/Ravenloft")).unwrap();
        std::fs::create_dir_all(dir.0.join("video/effects")).unwrap();
        std::fs::write(dir.0.join("video/ambience/Ravenloft/gate.webm"), b"x").unwrap();
        std::fs::write(dir.0.join("video/ambience/Ravenloft/notes.txt"), b"x").unwrap();
        std::fs::write(dir.0.join("video/effects/flash.webm"), b"xy").unwrap();
        std::fs::write(dir.0.join("video/effects/.partial.mp4"), b"x").unwrap();

        assert_eq!(list_folders(&dir.0), VideoFolders { ambience: vec!["Ravenloft".into()], effect: vec![] });
        let mut found = scan_files(&dir.0);
        found.sort();
        assert_eq!(
            found,
            [
                ("video/ambience/Ravenloft/gate.webm".to_string(), 1, VideoKind::Ambience),
                ("video/effects/flash.webm".to_string(), 2, VideoKind::Effect),
            ]
        );
    }

    #[test]
    fn a_video_knows_its_folder() {
        assert_eq!(folder_of(&new_video("a".into(), file("video/ambience/a.webm"), VideoKind::Ambience)), "");
        assert_eq!(
            folder_of(&new_video("a".into(), file("video/effects/Raven/Castle/a.webm"), VideoKind::Effect)),
            "Raven/Castle"
        );
    }

    /// Decision 0039: videos from before the two kinds move into `video/ambience/`, folders and
    /// all, and become ambience videos; a missing one just gets its path updated.
    #[test]
    fn older_projects_videos_move_into_the_ambience_folder() {
        let dir = TempDir::new();
        std::fs::create_dir_all(dir.0.join("video/Ravenloft")).unwrap();
        std::fs::write(dir.0.join("video/Ravenloft/gate.webm"), b"x").unwrap();
        std::fs::write(dir.0.join("video/flash.webm"), b"x").unwrap();
        let mut project = Project::new("P", crate::project::ProjectDefaults::default());
        // Files written before `kind` existed load as ambience; make one say "effect" to check
        // that the folder decides.
        project.videos = vec![
            new_video("gate".into(), file("video/Ravenloft/gate.webm"), VideoKind::Ambience),
            new_video("flash".into(), file("video/flash.webm"), VideoKind::Effect),
            new_video("gone".into(), file("video/gone.webm"), VideoKind::Ambience),
        ];

        assert!(move_into_kind_folders(&dir.0, &mut project).unwrap());
        let paths: Vec<_> = project.videos.iter().map(|v| (v.file.path.as_str(), v.kind)).collect();
        assert_eq!(
            paths,
            [
                ("video/ambience/Ravenloft/gate.webm", VideoKind::Ambience),
                ("video/ambience/flash.webm", VideoKind::Ambience),
                ("video/ambience/gone.webm", VideoKind::Ambience),
            ]
        );
        assert!(dir.0.join("video/ambience/Ravenloft/gate.webm").is_file());
        assert!(dir.0.join("video/ambience/flash.webm").is_file());
        // Done once: nothing left to move the next time.
        assert!(!move_into_kind_folders(&dir.0, &mut project).unwrap());
    }
}
