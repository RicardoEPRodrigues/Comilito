//! Looking at videos with ffprobe, and converting the ones that may not play to WebM
//! (decision 0035).
//!
//! WebKitGTK (Linux) plays video through GStreamer, and H.264 MP4 only plays there when the
//! right GStreamer packages are installed. WebM with VP8, VP9 or AV1 plays everywhere, so other
//! videos are offered for conversion with the ffmpeg Comilito already downloads for yt-dlp.
//!
//! - **Probing:** the ticker (lib.rs) runs `probe` on each video without `info`, one at a time.
//! - **Offers:** videos added in this session (files, drops, the folder, downloads) that turn
//!   out not to play everywhere are collected; once nothing is left to probe, the batch is
//!   offered (`convert-offer` event), converted straight away, or left, per Preferences.
//! - **Conversions** wait in a queue and run one at a time on a background thread, like
//!   downloads (`conversions-changed` event). They aren't kept across restarts: an unconverted
//!   video stays in the Library, marked "may not play", with a Convert button.

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    commands::{AppStateRef, lock},
    downloads::JobStatus,
    project::{Shelf, VideoInfo},
    state::{AppState, emit_state},
    storage, tools,
};

/// Whether every webview can play a file: WebM with VP8, VP9 or AV1. (A .mkv with VP9 has the
/// same insides as a WebM, but webviews go by the file type, so it's converted too.)
pub fn plays_everywhere(extension: &str, codec: &str) -> bool {
    extension.eq_ignore_ascii_case("webm") && matches!(codec, "vp8" | "vp9" | "av1")
}

/// Reads a video's container, codec, size and length with ffprobe.
pub fn probe(ffprobe: &Path, file: &Path) -> Result<VideoInfo, String> {
    let out = tools::hidden_command(ffprobe)
        .args(["-v", "error", "-select_streams", "v:0"])
        .args(["-show_entries", "stream=codec_name,width,height:format=format_name,duration"])
        .args(["-of", "json"])
        .arg(file)
        .output()
        .map_err(|e| format!("could not run ffprobe: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|_| "ffprobe gave an unexpected answer".to_string())?;
    parse_probe(&json, file)
}

fn parse_probe(json: &serde_json::Value, file: &Path) -> Result<VideoInfo, String> {
    let stream = &json["streams"][0];
    let codec = stream["codec_name"].as_str().ok_or("no video in this file")?.to_string();
    let extension = file.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    Ok(VideoInfo {
        container: json["format"]["format_name"].as_str().unwrap_or("").to_string(),
        width: stream["width"].as_u64().unwrap_or(0) as u32,
        height: stream["height"].as_u64().unwrap_or(0) as u32,
        // ffprobe writes the length as a string of seconds ("12.040000").
        duration_ms: json["format"]["duration"]
            .as_str()
            .and_then(|d| d.parse::<f64>().ok())
            .map(|s| (s * 1000.0) as u64)
            .unwrap_or(0),
        plays_everywhere: plays_everywhere(&extension, &codec),
        codec,
    })
}

// ---- Offering conversions ------------------------------------------------------------------

/// Where new videos came from, for the Convert dialog's first line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// Added with Add files, dropped on the window, or downloaded.
    Added,
    /// Found in the project's `video/` folder.
    Folder,
}

/// Videos added in this session, waiting to be looked at (`fresh`) or, once found not to play
/// everywhere, to be offered for conversion (`offer`).
#[derive(Debug, Default)]
pub struct Offers {
    pub fresh: HashMap<String, Origin>,
    pub offer: Vec<(String, Origin)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertOffer {
    pub ids: Vec<String>,
    /// "folder" when every video came from the project folder, otherwise "added".
    pub origin: Origin,
}

/// Remembers videos that were just added, so they're offered for conversion if they need it.
pub fn note_new(state: &mut AppState, ids: &[String], origin: Origin) {
    for id in ids {
        state.video_offers.fresh.insert(id.clone(), origin);
    }
}

/// After a probe: a fresh video that may not play joins the next offer.
pub fn probed(state: &mut AppState, id: &str, info: &VideoInfo) {
    if let Some(origin) = state.video_offers.fresh.remove(id)
        && !info.plays_everywhere
    {
        state.video_offers.offer.push((id.to_string(), origin));
    }
}

/// Called by the ticker when no video is left to probe: offers the collected videos, converts
/// them, or forgets them, per Preferences › Video › "Videos that may not play".
pub fn flush_offer(app: &AppHandle, state: &mut AppState) {
    if state.video_offers.offer.is_empty() {
        return;
    }
    let batch = std::mem::take(&mut state.video_offers.offer);
    let ids: Vec<String> = batch.iter().map(|(id, _)| id.clone()).collect();
    match state.prefs.video.convert.as_str() {
        "never" => {}
        "always" => {
            enqueue(state, &ids, false);
            emit_conversions(app, state);
        }
        _ => {
            let origin = if batch.iter().all(|(_, o)| *o == Origin::Folder) { Origin::Folder } else { Origin::Added };
            let _ = app.emit("convert-offer", ConvertOffer { ids, origin });
        }
    }
}

// ---- The conversion queue ------------------------------------------------------------------

/// One conversion, as the UI sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionJob {
    /// The video's id (a video is converted at most once at a time).
    pub id: String,
    pub name: String,
    pub status: JobStatus,
    /// 0.0–1.0 while running.
    pub progress: f32,
    pub message: Option<String>,
    #[serde(skip)]
    keep_original: bool,
}

/// Adds videos to the conversion queue (skipping ones already waiting) and wakes the worker.
pub fn enqueue(state: &mut AppState, ids: &[String], keep_original: bool) {
    let Some(open) = &state.open else { return };
    for id in ids {
        let Some(video) = open.project.video(id) else { continue };
        let waiting = state
            .conversions
            .iter()
            .any(|j| j.id == *id && matches!(j.status, JobStatus::Queued | JobStatus::Running));
        if waiting {
            continue;
        }
        state.conversions.retain(|j| j.id != *id);
        state.conversions.push(ConversionJob {
            id: id.clone(),
            name: video.name.clone(),
            status: JobStatus::Queued,
            progress: 0.0,
            message: None,
            keep_original,
        });
    }
    if let Some(tx) = &state.conversion_wakeup {
        let _ = tx.send(());
    }
}

pub fn emit_conversions(app: &AppHandle, state: &AppState) {
    let _ = app.emit("conversions-changed", &state.conversions);
}

fn update(app: &AppHandle, id: &str, f: impl FnOnce(&mut ConversionJob)) {
    let state = app.state::<Mutex<AppState>>();
    if let Ok(mut s) = state.lock() {
        if let Some(job) = s.conversions.iter_mut().find(|j| j.id == id) {
            f(job);
        }
        emit_conversions(app, &s);
    }
}

fn is_cancelled(app: &AppHandle, id: &str) -> bool {
    let state = app.state::<Mutex<AppState>>();
    state
        .lock()
        .map(|s| s.quitting || s.conversions.iter().any(|j| j.id == id && j.status == JobStatus::Cancelled))
        .unwrap_or(true)
}

/// What a conversion needs, read under the lock before it starts.
struct Work {
    id: String,
    dir: PathBuf,
    /// The video's stored path when the conversion started, to notice if it changed meanwhile.
    stored_path: String,
    input: PathBuf,
    duration_ms: u64,
    keep_original: bool,
    tools_dir: PathBuf,
}

/// The worker thread: waits to be woken, then converts queued videos one by one.
pub fn worker(app: AppHandle, wakeup: mpsc::Receiver<()>) {
    while wakeup.recv().is_ok() {
        loop {
            let work = {
                let state = app.state::<Mutex<AppState>>();
                let Ok(mut s) = state.lock() else { return };
                let s = &mut *s;
                let Some(job) = s.conversions.iter_mut().find(|j| j.status == JobStatus::Queued) else {
                    break;
                };
                let found = s.open.as_ref().and_then(|open| {
                    let video = open.project.video(&job.id)?;
                    Some(Work {
                        id: job.id.clone(),
                        dir: open.dir.clone(),
                        stored_path: video.file.path.clone(),
                        input: storage::absolute_audio_path(&open.dir, &video.file),
                        duration_ms: video.info.as_ref().map_or(0, |i| i.duration_ms),
                        keep_original: job.keep_original,
                        tools_dir: s.tools_dir.clone(),
                    })
                });
                match found {
                    Some(work) => {
                        job.status = JobStatus::Running;
                        emit_conversions(&app, s);
                        work
                    }
                    None => {
                        // Its project was closed, or the video removed.
                        job.status = JobStatus::Cancelled;
                        emit_conversions(&app, s);
                        continue;
                    }
                }
            };
            let result = convert(&app, &work);
            finish(&app, &work, result);
        }
    }
}

/// `video/Castle/gate.mp4` → the hidden working file `video/Castle/.gate.converting.webm`.
/// Hidden, so the folder scan doesn't pick up a half-written file.
fn working_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    input.with_file_name(format!(".{stem}.converting.webm"))
}

/// Runs ffmpeg: the first video stream to WebM (VP9), without sound or subtitles, keeping the
/// size. Returns the working file once it's complete.
fn convert(app: &AppHandle, work: &Work) -> Result<PathBuf, String> {
    let ffmpeg = tools::ffmpeg_path(&work.tools_dir).ok_or("TOOLS_MISSING")?;
    if !work.input.is_file() {
        return Err("the video's file is missing".into());
    }
    let output = working_path(&work.input);
    let mut cmd = tools::hidden_command(&ffmpeg);
    cmd.args(["-hide_banner", "-nostats", "-loglevel", "error", "-y", "-i"])
        .arg(&work.input)
        .args(["-map", "0:v:0", "-an", "-sn"])
        // Constant quality (crf) VP9; "good" with cpu-used 4 is a fair speed for the quality,
        // and row-mt uses every core.
        .args(["-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-deadline", "good", "-cpu-used", "4"])
        .args(["-row-mt", "1", "-pix_fmt", "yuv420p"])
        // Machine-readable progress on stdout: "out_time_us=12345678" lines.
        .args(["-progress", "pipe:1"])
        .arg(&output);
    tools::own_process_group(&mut cmd);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run ffmpeg: {e}"))?;
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let child = Arc::new(Mutex::new(child));
    if let Ok(mut s) = app.state::<Mutex<AppState>>().lock() {
        s.conversion_process = Some(child.clone());
    }

    let (app2, id2, total) = (app.clone(), work.id.clone(), work.duration_ms);
    let progress = thread::spawn(move || {
        let mut last = Instant::now() - Duration::from_secs(1);
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let Some(us) = line.strip_prefix("out_time_us=").and_then(|v| v.trim().parse::<u64>().ok()) else {
                continue;
            };
            if total > 0 && last.elapsed() > Duration::from_millis(400) {
                last = Instant::now();
                update(&app2, &id2, |j| j.progress = (us as f32 / 1000.0 / total as f32).clamp(0.0, 1.0));
            }
        }
    });
    let errors = thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut BufReader::new(stderr), &mut text);
        text
    });

    let result = loop {
        let Ok(mut process) = child.lock() else { break Err("busy".to_string()) };
        if is_cancelled(app, &work.id) {
            tools::kill_tree(&mut process);
            break Err("CANCELLED".into());
        }
        match process.try_wait() {
            Ok(Some(status)) if status.success() => break Ok(()),
            Ok(Some(_)) => break Err(String::new()),
            Ok(None) => {
                drop(process);
                thread::sleep(Duration::from_millis(200));
            }
            Err(e) => break Err(e.to_string()),
        }
    };
    if let Ok(mut s) = app.state::<Mutex<AppState>>().lock() {
        s.conversion_process = None;
    }
    let _ = progress.join();
    let stderr = errors.join().unwrap_or_default();
    match result {
        Ok(()) => Ok(output),
        Err(e) => {
            let _ = fs::remove_file(&output);
            if e.is_empty() {
                let line = stderr.lines().last().unwrap_or("ffmpeg could not convert it").to_string();
                Err(line)
            } else {
                Err(e)
            }
        }
    }
}

/// Puts the converted file in place of the original (trashed, or kept as a video of its own).
fn finish(app: &AppHandle, work: &Work, result: Result<PathBuf, String>) {
    let state = app.state::<Mutex<AppState>>();
    let Ok(mut s) = state.lock() else { return };
    if s.quitting {
        if let Ok(path) = &result {
            let _ = fs::remove_file(path);
        }
        return;
    }
    let tools_dir = s.tools_dir.clone();
    let mut problems = Vec::new();
    let outcome = result.and_then(|working| place(&mut s, work, &working, &tools_dir, &mut problems));
    let name = s.conversions.iter().find(|j| j.id == work.id).map(|j| j.name.clone()).unwrap_or_default();
    if let Some(job) = s.conversions.iter_mut().find(|j| j.id == work.id) {
        match &outcome {
            Ok(()) => {
                job.status = JobStatus::Done;
                job.progress = 1.0;
                job.message = None;
            }
            Err(e) if e == "CANCELLED" => job.status = JobStatus::Cancelled,
            Err(e) => {
                job.status = JobStatus::Failed;
                job.message = Some(if e == "TOOLS_MISSING" { "ffmpeg isn't installed".into() } else { e.clone() });
            }
        }
    }
    match &outcome {
        Ok(()) => {
            crate::logging::info(&format!("Converted \"{name}\" to WebM"));
            problems.push(format!("Converted \"{name}\"."));
            problems.extend(s.project_changed(Duration::ZERO));
        }
        Err(e) if e != "CANCELLED" => crate::logging::error(&format!("Converting \"{name}\" failed: {e}")),
        Err(_) => {}
    }
    emit_conversions(app, &s);
    emit_state(app, &s, problems);
}

fn place(s: &mut AppState, work: &Work, working: &Path, tools_dir: &Path, problems: &mut Vec<String>) -> Result<(), String> {
    let open = s.open.as_mut().filter(|o| o.dir == work.dir).ok_or("its project was closed")?;
    let video = open.project.video(&work.id).ok_or("the video was removed")?;
    if video.file.path != work.stored_path {
        let _ = fs::remove_file(working);
        return Err("the video's file changed meanwhile".into());
    }
    let original = work.input.clone();
    let stem = original.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into());
    let folder = original.parent().ok_or("no folder")?;
    let target = storage::unique_path(folder, Path::new(&format!("{stem}.webm")));
    fs::rename(working, &target).map_err(|e| format!("could not save the converted video: {e}"))?;
    let relative = target.strip_prefix(&work.dir).map_err(|e| e.to_string())?.to_path_buf();
    let mut file = storage::file_info(&work.dir, &relative)?;
    file.sha256 = None;
    let info = tools::ffprobe_path(tools_dir).and_then(|ffprobe| probe(&ffprobe, &target).ok());

    let old = video.clone();
    if let Some(video) = open.project.video_mut(&work.id) {
        video.file = file;
        video.info = info;
    }
    if work.keep_original {
        // The original stays in the folder as a video of its own (otherwise the folder scan
        // would add it back and offer to convert it again).
        let mut kept = crate::video::new_video(format!("{} (original)", old.name), old.file.clone());
        kept.info = old.info.clone();
        kept.tags = old.tags.clone();
        open.project.videos.push(kept);
    } else if let Err(e) = trash::delete(&original) {
        problems.push(format!("could not move the original to the trash: {e}"));
    }
    open.video_folders = crate::video::list_folders(&open.dir);
    Ok(())
}

/// Deletes working files left by conversions cut off by quitting.
pub fn clean_leftovers(dir: &Path) {
    let mut to_visit = vec![storage::shelf_root(dir, Shelf::Video)];
    while let Some(folder) = to_visit.pop() {
        let Ok(entries) = fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                to_visit.push(path);
            } else if name.starts_with('.') && name.ends_with(".converting.webm") {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

// ---- Commands -------------------------------------------------------------------------------

/// Converts videos to WebM in the background. `keep_originals` keeps each original as a video
/// of its own instead of moving it to the trash.
#[tauri::command]
pub fn convert_videos(ids: Vec<String>, keep_originals: bool, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    if tools::ffmpeg_path(&s.tools_dir).is_none() {
        return Err("TOOLS_MISSING".into());
    }
    enqueue(&mut s, &ids, keep_originals);
    emit_conversions(&app, &s);
    Ok(())
}

#[tauri::command]
pub fn cancel_conversion(id: String, app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    if let Some(job) = s.conversions.iter_mut().find(|j| j.id == id)
        && matches!(job.status, JobStatus::Queued | JobStatus::Running)
    {
        job.status = JobStatus::Cancelled;
    }
    emit_conversions(&app, &s);
    Ok(())
}

/// Removes finished, failed and cancelled conversions from the list.
#[tauri::command]
pub fn clear_conversions(app: AppHandle, state: AppStateRef) -> Result<(), String> {
    let mut s = lock(&state)?;
    s.conversions.retain(|j| matches!(j.status, JobStatus::Queued | JobStatus::Running));
    emit_conversions(&app, &s);
    Ok(())
}

#[tauri::command]
pub fn list_conversions(state: AppStateRef) -> Result<Vec<ConversionJob>, String> {
    Ok(lock(&state)?.conversions.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_webm_with_open_codecs_plays_everywhere() {
        assert!(plays_everywhere("webm", "vp9"));
        assert!(plays_everywhere("WEBM", "av1"));
        assert!(!plays_everywhere("mp4", "h264"));
        assert!(!plays_everywhere("mkv", "vp9"), "webviews go by the file type");
        assert!(!plays_everywhere("webm", "h264"));
    }

    #[test]
    fn reads_ffprobe_answers() {
        // As printed by ffprobe for an H.264 MP4.
        let json: serde_json::Value = serde_json::from_str(
            r#"{"streams":[{"codec_name":"h264","width":640,"height":360}],
                "format":{"format_name":"mov,mp4,m4a,3gp,3g2,mj2","duration":"4.000000"}}"#,
        )
        .unwrap();
        let info = parse_probe(&json, Path::new("in.mp4")).unwrap();
        assert_eq!((info.width, info.height, info.duration_ms), (640, 360, 4000));
        assert_eq!(info.codec, "h264");
        assert!(!info.plays_everywhere);

        let no_video: serde_json::Value = serde_json::from_str(r#"{"streams":[],"format":{}}"#).unwrap();
        assert!(parse_probe(&no_video, Path::new("a.webm")).is_err());
    }

    #[test]
    fn working_files_are_hidden_and_cleaned_up() {
        let dir = crate::storage::test_support::TempDir::new();
        let folder = dir.0.join("video/Castle");
        fs::create_dir_all(&folder).unwrap();
        let working = working_path(&folder.join("gate.mp4"));
        assert_eq!(working.file_name().unwrap(), ".gate.converting.webm");
        fs::write(&working, b"half").unwrap();
        fs::write(folder.join("gate.mp4"), b"video").unwrap();
        clean_leftovers(&dir.0);
        assert!(!working.exists());
        assert!(folder.join("gate.mp4").exists());
    }

    /// Converts a generated H.264 clip with the downloaded ffmpeg:
    /// `COMILITO_TOOLS=~/.local/share/com.comilito.app/tools cargo test converts_ -- --ignored`
    #[test]
    #[ignore]
    fn converts_a_real_video_with_the_downloaded_ffmpeg() {
        let tools_dir = PathBuf::from(std::env::var("COMILITO_TOOLS").expect("set COMILITO_TOOLS"));
        let ffmpeg = tools::ffmpeg_path(&tools_dir).unwrap();
        let ffprobe = tools::ffprobe_path(&tools_dir).unwrap();
        let dir = crate::storage::test_support::TempDir::new();
        let input = dir.0.join("in.mp4");
        let made = tools::hidden_command(&ffmpeg)
            .args(["-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=320x180:rate=25", "-t", "2", "-c:v", "libx264"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(made.success());
        let info = probe(&ffprobe, &input).unwrap();
        assert!(!info.plays_everywhere);

        let out = dir.0.join("out.webm");
        let status = tools::hidden_command(&ffmpeg)
            .args(["-loglevel", "error", "-y", "-i"])
            .arg(&input)
            .args(["-map", "0:v:0", "-an", "-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-deadline", "good", "-cpu-used", "4"])
            .arg(&out)
            .status()
            .unwrap();
        assert!(status.success());
        let converted = probe(&ffprobe, &out).unwrap();
        assert!(converted.plays_everywhere, "{converted:?}");
        assert_eq!((converted.width, converted.height), (320, 180));
    }
}
