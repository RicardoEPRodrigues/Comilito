//! Videos (decision 0035): the files in the project's `video/` folder and the Library commands
//! for them. Kept in one module, apart from the audio code, so the feature stays easy to find
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
    project::{FileInfo, Project, Shelf, Video},
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

/// The Library's video folders (subfolders of `video/`), like "Ravenloft/Castle", sorted.
pub fn list_folders(dir: &Path) -> Vec<String> {
    storage::subfolders(&storage::shelf_root(dir, Shelf::Video))
}

/// Every video file under `video/`, as (path relative to the project, size).
pub fn scan_files(dir: &Path) -> Vec<(String, u64)> {
    storage::files_on_shelf(dir, Shelf::Video, &VIDEO_EXTENSIONS)
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
pub fn new_video(name: String, file: FileInfo) -> Video {
    Video {
        id: crate::project::new_id("vid"),
        name,
        file,
        info: None,
        tags: Vec::new(),
        favourite: false,
        source: None,
        last_used: None,
    }
}

/// The Library folder a video's file is in: the path between `video/` and the file name.
pub fn folder_of(video: &Video) -> String {
    let rest = video.file.path.strip_prefix("video/").unwrap_or(&video.file.path);
    rest.rsplit_once('/').map(|(folder, _)| folder.to_string()).unwrap_or_default()
}

fn name_from(path: &Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Video".into())
}

// ---- Commands -------------------------------------------------------------------------------

/// Copies video files into `video/<folder>/` and adds them to the library. Returns their ids.
#[tauri::command]
pub fn add_videos(
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
        match storage::copy_into_shelf(&open.dir, Shelf::Video, &folder, &path) {
            Ok(file) => {
                let video = new_video(name_from(&path), file);
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

/// Moves videos into another Library folder (their files move under `video/<folder>/`).
#[tauri::command]
pub fn move_videos(ids: Vec<String>, folder: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    let mut problems = Vec::new();
    for id in ids {
        let Some(video) = open.project.video(&id) else { continue };
        if folder_of(video) == folder || open.missing.contains(&id) {
            continue;
        }
        match storage::move_into_shelf(&open.dir, &video.file, Shelf::Video, &folder) {
            Ok(file) => {
                if let Some(video) = open.project.video_mut(&id) {
                    video.file = file;
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

/// Adds a video to a scene's playlist (a video dragged onto a scene or a tapestry entry).
#[tauri::command]
pub fn add_video_to_scene(scene_id: String, video_id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    let open = s.open_mut()?;
    if open.project.video(&video_id).is_none() {
        return Err("that video no longer exists".into());
    }
    let scene = open
        .project
        .scenes
        .iter_mut()
        .find(|sc| sc.id == scene_id)
        .ok_or("that scene no longer exists")?;
    let playlist = scene.video.get_or_insert_with(Default::default);
    if !playlist.videos.contains(&video_id) {
        playlist.videos.push(video_id);
    }
    // A scene with videos of its own has nothing to hide.
    scene.hides_video = false;
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
    let file = storage::copy_into_shelf(&open.dir, Shelf::Video, &folder, source)?;
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

    #[test]
    fn finds_video_files_and_folders() {
        let dir = TempDir::new();
        let folder = dir.0.join("video/Ravenloft");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("gate.webm"), b"x").unwrap();
        std::fs::write(folder.join("notes.txt"), b"x").unwrap();
        std::fs::write(dir.0.join("video/.partial.mp4"), b"x").unwrap();

        assert_eq!(list_folders(&dir.0), ["Ravenloft"]);
        assert_eq!(scan_files(&dir.0), [("video/Ravenloft/gate.webm".to_string(), 1)]);
    }

    #[test]
    fn a_video_knows_its_folder() {
        let file = |path: &str| FileInfo {
            path: path.into(),
            size: 0,
            modified: String::new(),
            sha256: None,
            loudness: None,
        };
        assert_eq!(folder_of(&new_video("a".into(), file("video/a.webm"))), "");
        assert_eq!(folder_of(&new_video("a".into(), file("video/Raven/Castle/a.webm"))), "Raven/Castle");
    }
}
