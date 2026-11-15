//! Downloading audio from a URL into the project (decision 0019). Nothing is ever streamed:
//! the audio is saved into `audio/<category>/<folder>/` and becomes an ordinary sound.
//!
//! - A **direct link** to an audio file is fetched over HTTP (ureq).
//! - **Any other page** (YouTube and the many sites yt-dlp supports) goes through yt-dlp,
//!   asking for audio only, preferring AAC in M4A (it plays without conversion; symphonia
//!   can't decode Opus, so Opus-only sources are converted by ffmpeg).
//!
//! Downloads wait in a queue and run one at a time on a background thread. The queue lives
//! in `AppState::downloads`, and every change is sent to the UI as a `downloads-changed` event.
//!
//! Unfinished downloads survive quitting: the queue is written to `downloads.json` in the app
//! data folder, and a download resumes the next time its project is opened. Partial data is
//! kept in the cache folder (`*.part` files), so yt-dlp and direct links continue where they
//! stopped instead of starting over.

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::{
    crop::{self, CutOptions},
    project::{Category, Shelf, Sound, Source, TimeRange},
    state::{AppState, emit_state},
    storage, tools,
};

/// What the "Add audio from a URL" dialog shows after looking at a link.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    /// "direct" (an audio file) or "page" (handled by yt-dlp).
    pub kind: String,
    pub url: String,
    pub title: String,
    pub site: String,
    pub duration_ms: Option<u64>,
    pub thumbnail: Option<String>,
    /// Estimated size of the whole audio, in bytes.
    pub size: Option<u64>,
    /// E.g. "AAC 128 kbps".
    pub quality: Option<String>,
    /// File extension the audio will have.
    pub ext: String,
    /// A direct link to a video file (decision 0035): only "download as video" makes sense.
    pub is_video: bool,
    /// For pages: the picture alone at each resolution limit, for "download as video".
    pub video_options: Vec<VideoOption>,
}

/// What "download as video" would fetch with a given resolution limit (decision 0035).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoOption {
    /// The limit chosen in the dialog: 720, 1080, 1440, or 0 for the best there is.
    pub max_height: u32,
    /// The height of the format that would be picked.
    pub height: u32,
    /// "vp9", "av1", "h264"...
    pub codec: String,
    pub ext: String,
    pub size: Option<u64>,
    /// WebM with an open codec: plays here without converting.
    pub plays_everywhere: bool,
}

/// yt-dlp's codec strings ("vp09.00.40.08", "avc1.640028", "av01.0.08M.08") as short names.
fn short_codec(vcodec: &str) -> String {
    let v = vcodec.to_lowercase();
    let name = if v.starts_with("vp09") || v.starts_with("vp9") {
        "vp9"
    } else if v.starts_with("vp08") || v.starts_with("vp8") {
        "vp8"
    } else if v.starts_with("av01") || v.starts_with("av1") {
        "av1"
    } else if v.starts_with("avc") || v.starts_with("h264") {
        "h264"
    } else if v.starts_with("hev") || v.starts_with("hvc") || v.starts_with("h265") {
        "h265"
    } else {
        v.split('.').next().unwrap_or(&v)
    };
    name.to_string()
}

/// The best picture-only format at each resolution limit, preferring ones that play
/// everywhere (WebM with VP9 or AV1), then the highest resolution, then the highest bitrate.
fn video_options(formats: &[serde_json::Value], duration_ms: Option<u64>) -> Vec<VideoOption> {
    let candidates: Vec<VideoOption> = formats
        .iter()
        .filter(|f| f["acodec"].as_str() == Some("none") && f["vcodec"].as_str().is_some_and(|v| v != "none"))
        .filter_map(|f| {
            let height = f["height"].as_u64()? as u32;
            let codec = short_codec(f["vcodec"].as_str()?);
            let ext = f["ext"].as_str().unwrap_or("").to_string();
            let tbr = f["tbr"].as_f64();
            let size = f["filesize"]
                .as_u64()
                .or(f["filesize_approx"].as_u64())
                .or_else(|| Some((tbr? * 1000.0 / 8.0 * duration_ms? as f64 / 1000.0) as u64));
            Some(VideoOption {
                max_height: 0,
                height,
                plays_everywhere: crate::video_convert::plays_everywhere(&ext, &codec),
                codec,
                ext,
                size,
            })
        })
        .collect();
    [720, 1080, 1440, 0]
        .into_iter()
        .filter_map(|limit| {
            let best = candidates
                .iter()
                .filter(|o| limit == 0 || o.height <= limit)
                .max_by_key(|o| (o.plays_everywhere, o.height, o.size.unwrap_or(0)))?;
            Some(VideoOption { max_height: limit, ..best.clone() })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub kind: String,
    pub title: String,
    pub site: String,
    pub ext: String,
    pub category: Category,
    pub folder: String,
    pub name: String,
    pub range: Option<TimeRange>,
    /// "Download again": put the new file into this existing sound instead of adding one.
    #[serde(default)]
    pub replace: Option<String>,
    /// Download the picture only, as a video (decision 0035), up to this height (0: the best).
    /// The video goes to `video/<folder>/` and `category` is ignored.
    #[serde(default)]
    pub video_height: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

/// One download in the queue, as the UI sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJob {
    pub id: String,
    pub name: String,
    pub url: String,
    pub site: String,
    pub status: JobStatus,
    /// 0.0–1.0, while running.
    pub progress: f32,
    /// What's happening ("converting…") or what went wrong.
    pub message: Option<String>,
    /// The new sound, once done.
    pub sound_id: Option<String>,
    #[serde(skip)]
    request: Option<DownloadRequest>,
    #[serde(skip)]
    project_dir: PathBuf,
}

/// An unfinished download as written to `downloads.json`, to resume it after a restart.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDownload {
    /// Kept so the partial data in the cache folder (named after the id) is found again.
    pub id: String,
    pub project_dir: PathBuf,
    pub request: DownloadRequest,
}

pub const SAVED_FILE: &str = "downloads.json";

/// Reads the unfinished downloads left by the last run. Downloads for projects that no longer
/// exist are dropped (and their partial data deleted), as are the cache folders of downloads
/// that aren't in the list any more.
pub fn load_saved(data_dir: &Path, cache_dir: &Path) -> Vec<SavedDownload> {
    let saved: Vec<SavedDownload> = fs::read_to_string(data_dir.join(SAVED_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let saved: Vec<SavedDownload> = saved
        .into_iter()
        .filter(|d| d.project_dir.join(storage::PROJECT_FILE).is_file())
        .collect();
    if let Ok(entries) = fs::read_dir(cache_dir.join("downloads")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !saved.iter().any(|d| d.id == name) {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    saved
}

/// Writes the downloads that haven't finished (queued or running in this run, plus those
/// waiting for their project to be opened) to `downloads.json`, or removes it when there are
/// none. Called whenever a download is added, finishes or is cancelled.
pub fn save_unfinished(state: &AppState) {
    let mut list: Vec<SavedDownload> = state
        .downloads
        .iter()
        .filter(|j| matches!(j.status, JobStatus::Queued | JobStatus::Running))
        .filter_map(|j| {
            Some(SavedDownload {
                id: j.id.clone(),
                project_dir: j.project_dir.clone(),
                request: j.request.clone()?,
            })
        })
        .collect();
    list.extend(state.waiting_downloads.iter().cloned());
    let file = state.data_dir.join(SAVED_FILE);
    let result = if list.is_empty() {
        match fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    } else {
        serde_json::to_string_pretty(&list)
            .map_err(|e| e.to_string())
            .and_then(|json| fs::write(&file, json).map_err(|e| e.to_string()))
    };
    if let Err(e) = result {
        crate::logging::warn(&format!("could not remember the unfinished downloads: {e}"));
    }
}

/// Puts the saved downloads of the project just opened back in the queue. Returns how many.
pub fn resume_for(state: &mut AppState, dir: &Path) -> usize {
    // `partition` splits a list in two by a test: (those that pass, those that don't).
    let (resume, keep): (Vec<_>, Vec<_>) =
        std::mem::take(&mut state.waiting_downloads).into_iter().partition(|d| d.project_dir == dir);
    state.waiting_downloads = keep;
    let count = resume.len();
    for saved in resume {
        state.downloads.push(DownloadJob {
            id: saved.id,
            name: saved.request.name.clone(),
            url: saved.request.url.clone(),
            site: saved.request.site.clone(),
            status: JobStatus::Queued,
            progress: 0.0,
            message: Some("resuming…".into()),
            sound_id: None,
            request: Some(saved.request),
            project_dir: saved.project_dir,
        });
    }
    if count > 0 {
        crate::logging::info(&format!("Resuming {count} unfinished download(s)"));
        if let Some(tx) = &state.download_wakeup {
            let _ = tx.send(());
        }
    }
    count
}

/// Audio formats a direct link may point at.
const DIRECT_EXTENSIONS: [&str; 7] = storage::AUDIO_EXTENSIONS;

fn direct_extension(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let last = path.rsplit('/').next()?;
    let ext = last.rsplit_once('.')?.1.to_lowercase();
    DIRECT_EXTENSIONS.contains(&ext.as_str()).then_some(ext)
}

fn direct_video_extension(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let last = path.rsplit('/').next()?;
    let ext = last.rsplit_once('.')?.1.to_lowercase();
    crate::video::VIDEO_EXTENSIONS.contains(&ext.as_str()).then_some(ext)
}

fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .map(|h| h.trim_start_matches("www.").to_string())
        .unwrap_or_default()
}

/// Looks at a URL: a direct audio file, or a page yt-dlp understands.
pub fn probe(tools_dir: &Path, url: &str) -> Result<Probe, String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("that doesn't look like a web address".into());
    }

    // A direct link to a video file: downloaded as a video (decision 0035).
    if let Some(ext) = direct_video_extension(url) {
        let response = ureq::head(url).call().map_err(|e| format!("couldn't reach that link: {e}"))?;
        let size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let file = url.split(['?', '#']).next().unwrap_or(url).rsplit('/').next().unwrap_or("video");
        return Ok(Probe {
            kind: "direct".into(),
            url: url.into(),
            title: percent_decode(file.rsplit_once('.').map_or(file, |(stem, _)| stem)),
            site: host_of(url),
            duration_ms: None,
            thumbnail: None,
            size,
            quality: Some(ext.to_uppercase()),
            ext,
            is_video: true,
            video_options: Vec::new(),
        });
    }

    if let Some(ext) = direct_extension(url) {
        let response = ureq::head(url).call().map_err(|e| format!("couldn't reach that link: {e}"))?;
        let size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        let file = url.split(['?', '#']).next().unwrap_or(url).rsplit('/').next().unwrap_or("audio");
        let title = percent_decode(file.rsplit_once('.').map_or(file, |(stem, _)| stem));
        return Ok(Probe {
            kind: "direct".into(),
            url: url.into(),
            title,
            site: host_of(url),
            duration_ms: None,
            thumbnail: None,
            size,
            quality: Some(ext.to_uppercase()),
            ext,
            is_video: false,
            video_options: Vec::new(),
        });
    }

    let yt_dlp = tools::yt_dlp_path(tools_dir);
    if !yt_dlp.is_file() {
        return Err("TOOLS_MISSING".into());
    }
    let out = tools::hidden_command(&yt_dlp)
        .args(["-J", "--no-playlist", "--no-warnings", "--skip-download"])
        .arg(url)
        .output()
        .map_err(|e| format!("could not run yt-dlp: {e}"))?;
    if !out.status.success() {
        let message = String::from_utf8_lossy(&out.stderr);
        let line = message.lines().rev().find(|l| l.contains("ERROR")).unwrap_or("no audio found at that address");
        return Err(line.trim_start_matches("ERROR:").trim().to_string());
    }
    let info: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|_| "yt-dlp gave an unexpected answer".to_string())?;
    let duration_ms = info["duration"].as_f64().map(|s| (s * 1000.0) as u64);

    // The best audio-only format, preferring M4A (plays without conversion).
    let formats = info["formats"].as_array().cloned().unwrap_or_default();
    let audio_only = formats
        .iter()
        .filter(|f| f["vcodec"].as_str() == Some("none") && f["acodec"].as_str().is_some_and(|a| a != "none"));
    let best = audio_only.max_by(|a, b| {
        let score = |f: &serde_json::Value| {
            (f["ext"].as_str() == Some("m4a"), f["abr"].as_f64().unwrap_or(0.0) as i64)
        };
        score(a).cmp(&score(b))
    });
    let (quality, size) = match best {
        Some(f) => {
            let codec = f["acodec"].as_str().unwrap_or("");
            let codec = if codec.starts_with("mp4a") { "AAC" } else if codec.starts_with("opus") { "Opus" } else { codec };
            let abr = f["abr"].as_f64();
            let size = f["filesize"]
                .as_u64()
                .or(f["filesize_approx"].as_u64())
                .or_else(|| Some((abr? * 1000.0 / 8.0 * duration_ms? as f64 / 1000.0) as u64));
            (Some(match abr {
                Some(k) => format!("{codec} {} kbps", k.round()),
                None => codec.to_string(),
            }), size)
        }
        None => (None, None),
    };

    Ok(Probe {
        kind: "page".into(),
        url: info["webpage_url"].as_str().unwrap_or(url).to_string(),
        title: info["title"].as_str().unwrap_or("Downloaded audio").to_string(),
        site: info["extractor_key"]
            .as_str()
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| host_of(url)),
        duration_ms,
        thumbnail: info["thumbnail"].as_str().map(String::from),
        size,
        quality,
        ext: "m4a".into(),
        is_video: false,
        video_options: video_options(&formats, duration_ms),
    })
}

/// Decodes `%20`-style escapes in a file name taken from a URL.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Adds a download to the queue and wakes the worker.
pub fn enqueue(state: &mut AppState, request: DownloadRequest) -> Result<String, String> {
    let dir = state.open_ref()?.dir.clone();
    let id = crate::project::new_id("dl");
    state.downloads.push(DownloadJob {
        id: id.clone(),
        name: request.name.clone(),
        url: request.url.clone(),
        site: request.site.clone(),
        status: JobStatus::Queued,
        progress: 0.0,
        message: None,
        sound_id: None,
        request: Some(request),
        project_dir: dir,
    });
    if let Some(tx) = &state.download_wakeup {
        let _ = tx.send(());
    }
    save_unfinished(state);
    Ok(id)
}

pub fn emit_downloads(app: &AppHandle, state: &AppState) {
    let _ = app.emit("downloads-changed", &state.downloads);
}

/// The worker thread: waits to be woken, then runs queued downloads one by one.
pub fn worker(app: AppHandle, wakeup: mpsc::Receiver<()>) {
    // `recv` blocks until someone sends; it fails only when the sender is gone (app exit).
    while wakeup.recv().is_ok() {
        loop {
            let next = {
                let state = app.state::<Mutex<AppState>>();
                let Ok(mut s) = state.lock() else { return };
                let Some(job) = s.downloads.iter_mut().find(|j| j.status == JobStatus::Queued) else {
                    break;
                };
                job.status = JobStatus::Running;
                job.message = None;
                let next = (job.id.clone(), job.request.clone(), job.project_dir.clone());
                emit_downloads(&app, &s);
                next
            };
            let (id, Some(request), dir) = next else { continue };
            let result = run(&app, &id, &request, &dir);
            finish(&app, &id, &request, &dir, result);
        }
    }
}

/// Updates one job (progress or message) and tells the UI.
fn update(app: &AppHandle, id: &str, f: impl FnOnce(&mut DownloadJob)) {
    let state = app.state::<Mutex<AppState>>();
    if let Ok(mut s) = state.lock() {
        if let Some(job) = s.downloads.iter_mut().find(|j| j.id == id) {
            f(job);
        }
        emit_downloads(app, &s);
    }
}

fn is_cancelled(app: &AppHandle, id: &str) -> bool {
    let state = app.state::<Mutex<AppState>>();
    state
        .lock()
        .map(|s| s.downloads.iter().any(|j| j.id == id && j.status == JobStatus::Cancelled))
        .unwrap_or(true)
}

fn temp_dir_for(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("downloads")
        .join(id);
    fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Downloads into a temporary folder and returns the finished file.
fn run(app: &AppHandle, id: &str, request: &DownloadRequest, _dir: &Path) -> Result<PathBuf, String> {
    let tmp = temp_dir_for(app, id)?;
    // A download stopped by quitting may have left files behind. Only partial downloads
    // (`.part`, and yt-dlp's `.ytdl` notes about them) can be continued; anything else may be
    // half-written (a conversion that was cut off) and is deleted. A part of a video is cut
    // by ffmpeg, which can't continue, so that starts over.
    clean_leftovers(&tmp, request.range.is_none());
    let tools_dir = app.state::<Mutex<AppState>>().lock().map_err(|_| "busy")?.tools_dir.clone();

    if request.kind == "direct" && request.video_height.is_some() {
        // A video file: nothing to check or cut (cutting is for audio).
        let file = tmp.join(format!("video.{}", request.ext));
        fetch_direct(app, id, &request.url, &file)?;
        return Ok(file);
    }
    if request.kind == "direct" {
        let file = tmp.join(format!("audio.{}", request.ext));
        fetch_direct(app, id, &request.url, &file)?;
        // Refuse files that aren't really audio (an HTML error page with a .mp3 name...).
        kira::sound::streaming::StreamingSoundData::from_file(&file)
            .map_err(|_| "the link didn't lead to an audio file Comilito can play".to_string())?;
        if let Some(range) = request.range {
            let ffmpeg = tools::ffmpeg_path(&tools_dir).ok_or("TOOLS_MISSING")?;
            update(app, id, |j| j.message = Some("cutting…".into()));
            let cut = tmp.join(format!("cut.{}", request.ext));
            crop::cut(
                &ffmpeg,
                &file,
                &cut,
                CutOptions { start_ms: range.start_ms, end_ms: range.end_ms, fade_edges: false, exact: false },
            )?;
            return Ok(cut);
        }
        return Ok(file);
    }

    let yt_dlp = tools::yt_dlp_path(&tools_dir);
    if !yt_dlp.is_file() || tools::ffmpeg_path(&tools_dir).is_none() {
        return Err("TOOLS_MISSING".into());
    }
    let mut cmd = tools::hidden_command(&yt_dlp);
    cmd.arg(&request.url)
        .args(["--no-playlist", "--newline", "--no-warnings", "--no-mtime", "--color", "never"]);
    match request.video_height {
        // The picture only (decision 0035), preferring WebM so it plays without converting.
        Some(height) => {
            let limit = if height > 0 { format!("[height<={height}]") } else { String::new() };
            cmd.args(["-f", &format!("bv{limit}[ext=webm]/bv{limit}/b{limit}")])
                .arg("-o")
                .arg(tmp.join("video.%(ext)s"));
        }
        None => {
            cmd.args(["-f", "bestaudio[ext=m4a]/bestaudio/best"])
                .args(["-x", "--audio-format", "m4a", "--audio-quality", "192K"])
                .arg("-o")
                .arg(tmp.join("audio.%(ext)s"));
        }
    }
    cmd.arg("--ffmpeg-location")
        .arg(tools::ffmpeg_dir(&tools_dir))
        .args(["--progress-template", "download:COMILITO %(progress._percent_str)s"]);
    if let Some(r) = request.range {
        // A part of a video is fetched by ffmpeg, which reports no percentage: say so instead
        // of showing a progress bar stuck at 0%.
        update(app, id, |j| j.message = Some("downloading the selected part…".into()));
        cmd.arg("--download-sections")
            .arg(format!("*{:.3}-{:.3}", r.start_ms as f64 / 1000.0, r.end_ms as f64 / 1000.0));
    }
    tools::own_process_group(&mut cmd);
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run yt-dlp: {e}"))?;
    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    // The child is shared with the app state so quitting can stop it: a child process isn't
    // stopped when its parent exits, and a stray yt-dlp would keep writing into the folder
    // the resumed download uses next time. `Arc<Mutex<…>>` is a shared, locked box (like a
    // `shared_ptr` to an object with its own mutex in C++).
    let child = Arc::new(Mutex::new(child));
    if let Ok(mut s) = app.state::<Mutex<AppState>>().lock() {
        s.download_process = Some(child.clone());
    }
    let section_ms = request.range.map(|r| r.end_ms.saturating_sub(r.start_ms));
    let wanted: &[&str] = if request.video_height.is_some() { &crate::video::VIDEO_EXTENSIONS } else { &storage::AUDIO_EXTENSIONS };
    let result = watch_yt_dlp(app, id, &tmp, &child, section_ms, wanted, stdout, stderr);
    if let Ok(mut s) = app.state::<Mutex<AppState>>().lock() {
        s.download_process = None;
    }
    result
}

/// Follows a running yt-dlp until it ends (or the download is cancelled) and returns the
/// audio file it left. `section_ms` is the length of the part being downloaded, if only a
/// part is: ffmpeg downloads that, and its progress is worked out from its `time=` output.
fn watch_yt_dlp(
    app: &AppHandle,
    id: &str,
    tmp: &Path,
    child: &Mutex<Child>,
    section_ms: Option<u64>,
    wanted: &[&str],
    stdout: impl Read + Send + 'static,
    stderr: impl Read + Send + 'static,
) -> Result<PathBuf, String> {
    // Read yt-dlp's output on other threads, so this one can watch for "cancel". Progress
    // lines can arrive on stdout or stderr depending on the version, so both are read; error
    // lines from stderr are kept for the message if the download fails.
    let errors = std::sync::Arc::new(Mutex::new(String::new()));
    let readers: Vec<_> = [Box::new(stdout) as Box<dyn Read + Send>, Box::new(stderr) as Box<dyn Read + Send>]
    .into_iter()
    .map(|stream| {
        let (app2, id2, errors) = (app.clone(), id.to_string(), errors.clone());
        thread::spawn(move || {
            let mut last = Instant::now() - Duration::from_secs(1);
            for line in output_lines(stream) {
                if let Some(rest) = line.trim().strip_prefix("COMILITO") {
                    // "N/A" (unknown size) is skipped rather than shown as 0%.
                    let Ok(percent) = rest.trim().trim_end_matches('%').trim().parse::<f32>() else {
                        continue;
                    };
                    if last.elapsed() > Duration::from_millis(250) || percent >= 100.0 {
                        last = Instant::now();
                        update(&app2, &id2, |j| j.progress = (percent / 100.0).clamp(0.0, 1.0));
                    }
                } else if let (Some(total), Some(done)) = (section_ms.filter(|&t| t > 0), ffmpeg_time_ms(&line)) {
                    if last.elapsed() > Duration::from_millis(250) {
                        last = Instant::now();
                        update(&app2, &id2, |j| {
                            j.progress = (done as f32 / total as f32).clamp(0.0, 1.0);
                            // Replaces "downloading the selected part…" with the percentage.
                            j.message = None;
                        });
                    }
                } else if line.contains("[ExtractAudio]") || line.contains("[Merger]") {
                    update(&app2, &id2, |j| {
                        j.progress = 1.0;
                        j.message = Some("converting…".into());
                    });
                } else if line.contains("ERROR") {
                    if let Ok(mut e) = errors.lock() {
                        *e = line.clone();
                    }
                }
            }
        })
    })
    .collect();

    let status = loop {
        let mut process = child.lock().map_err(|_| "busy")?;
        if is_cancelled(app, id) {
            tools::kill_tree(&mut process);
            return Err("CANCELLED".into());
        }
        match process.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None => {
                // Unlock before sleeping, so quitting can take the lock to stop the process.
                drop(process);
                thread::sleep(Duration::from_millis(200));
            }
        }
    };
    for reader in readers {
        let _ = reader.join();
    }
    if !status.success() {
        let line = errors.lock().map(|e| e.clone()).unwrap_or_default();
        let line = if line.is_empty() { "the download failed".to_string() } else { line };
        return Err(line.trim_start_matches("ERROR:").trim().to_string());
    }
    // yt-dlp chose the final name's extension; find the file it left.
    fs::read_dir(tmp)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| wanted.contains(&e.to_string_lossy().to_lowercase().as_str())))
        .ok_or_else(|| "yt-dlp finished but left no file".to_string())
}

/// Splits a program's output into lines at `\n` *and* `\r`. ffmpeg redraws its status line
/// in place by ending it with `\r` (carriage return: back to the start of the line) instead
/// of `\n`, so `BufRead::lines` would only hand over all its updates glued together when
/// ffmpeg finishes. Windows' `\r\n` line ends just give an empty piece, which is skipped.
fn output_lines(stream: impl Read) -> impl Iterator<Item = String> {
    BufReader::new(stream)
        .split(b'\r')
        .map_while(Result::ok)
        .flat_map(|chunk| {
            String::from_utf8_lossy(&chunk)
                .split('\n')
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|line| !line.trim().is_empty())
}

/// The `time=` of an ffmpeg status line (`size= 206KiB time=00:01:13.50 bitrate=…`), in
/// milliseconds: how much audio has been written so far, counted from the start of the part.
/// `None` for other lines, and for `time=N/A` (before anything is written).
fn ffmpeg_time_ms(line: &str) -> Option<u64> {
    let value = line.split("time=").nth(1)?.split_whitespace().next()?;
    // "-00:00:00.02" can appear at the very start; nothing has been written yet.
    if value.starts_with('-') {
        return None;
    }
    let mut parts = value.split(':');
    let (h, m, s) = (parts.next()?, parts.next()?, parts.next()?);
    let seconds = h.parse::<f64>().ok()? * 3600.0 + m.parse::<f64>().ok()? * 60.0 + s.parse::<f64>().ok()?;
    Some((seconds * 1000.0).round() as u64)
}

/// Downloads a direct link into `dest`, through `dest.part` so an interrupted download can
/// continue: if a `.part` file is there, only the rest is asked for (an HTTP `Range` request).
/// Servers that don't support that send the whole file again, which then starts over.
fn fetch_direct(app: &AppHandle, id: &str, url: &str, dest: &Path) -> Result<(), String> {
    let part = dest.with_extension(format!(
        "{}.part",
        dest.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default()
    ));
    let already = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let mut request = ureq::get(url);
    if already > 0 {
        request = request.header("Range", format!("bytes={already}-"));
    }
    let mut response = request.call().map_err(|e| format!("couldn't download: {e}"))?;
    // 206 "Partial Content": the server sent only the rest, so append to what we have.
    let resuming = already > 0 && response.status().as_u16() == 206;
    let rest: u64 = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut done = if resuming { already } else { 0 };
    let total = if rest > 0 { done + rest } else { 0 };
    let mut reader = response.body_mut().with_config().limit(u64::MAX).reader();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resuming)
        .truncate(!resuming)
        .open(&part)
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut last = Instant::now();
    loop {
        if is_cancelled(app, id) {
            return Err("CANCELLED".into());
        }
        let n = reader.read(&mut buffer).map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if total > 0 && last.elapsed() > Duration::from_millis(250) {
            last = Instant::now();
            update(app, id, |j| j.progress = done as f32 / total as f32);
        }
    }
    drop(file);
    fs::rename(&part, dest).map_err(|e| e.to_string())
}

/// Deletes what an interrupted download left in its cache folder, except (when `keep_partial`)
/// the partial downloads that can be continued.
fn clean_leftovers(tmp: &Path, keep_partial: bool) {
    let Ok(entries) = fs::read_dir(tmp) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let resumable = name.ends_with(".part") || name.ends_with(".ytdl");
        if !(keep_partial && resumable) {
            let _ = fs::remove_file(&path);
        }
    }
}

/// Moves the finished file into the project and adds it to the library.
fn finish(app: &AppHandle, id: &str, request: &DownloadRequest, dir: &Path, result: Result<PathBuf, String>) {
    // Quitting stops yt-dlp, which looks like a failure from here. Leave everything as it is
    // (partial data and `downloads.json`), so the download resumes next time.
    if app.state::<Mutex<AppState>>().lock().map(|s| s.quitting).unwrap_or(true) {
        return;
    }
    let placed = result.and_then(|file| {
        let ext = file.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| request.ext.clone());
        // A video goes to `video/<folder>/` (decision 0035), audio to its category's folder.
        let shelf = if request.video_height.is_some() { Shelf::Video } else { request.category.into() };
        let folder = storage::shelf_folder_path(dir, shelf, &request.folder);
        fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let name = storage::safe_file_name(&request.name);
        let target = storage::unique_path(&folder, Path::new(&format!("{}.{ext}", if name.is_empty() { "download" } else { &name })));
        move_file(&file, &target)?;
        Ok(target)
    });
    if let Ok(tmp) = temp_dir_for(app, id) {
        let _ = fs::remove_dir_all(tmp);
    }

    let state = app.state::<Mutex<AppState>>();
    let Ok(mut s) = state.lock() else { return };
    let mut problems = Vec::new();
    let outcome: Result<Option<String>, String> = placed.and_then(|target| {
        // Add it now if its project is still open; otherwise the folder watcher adds it the
        // next time the project opens (without the source information).
        let Some(open) = s.open.as_mut().filter(|o| o.dir == dir) else {
            return Ok(None);
        };
        let relative = target.strip_prefix(dir).map_err(|e| e.to_string())?.to_path_buf();
        let info = storage::file_info(dir, &relative)?;
        let source = Source {
            url: request.url.clone(),
            title: Some(request.title.clone()),
            site: Some(request.site.clone()),
            downloaded_at: storage::format_time(std::time::SystemTime::now()),
            range: request.range,
        };
        open.folders = storage::list_folders(dir);
        open.video_folders = crate::video::list_folders(dir);
        if request.video_height.is_some() {
            // "Download again" refills an existing video; otherwise a new one is added.
            if let Some(id) = &request.replace
                && let Some(existing) = open.project.video_mut(id)
            {
                existing.file = info;
                existing.info = None;
                existing.source = Some(source);
                open.missing.retain(|m| m != id);
                open.analysis_failed.retain(|m| m != id);
                return Ok(Some(id.clone()));
            }
            let mut video = crate::video::new_video(request.name.trim().to_string(), info);
            video.source = Some(source);
            let video_id = video.id.clone();
            open.project.videos.push(video);
            return Ok(Some(video_id));
        }
        // "Download again" refills an existing sound (typically one whose file went missing).
        if let Some(id) = &request.replace
            && let Some(existing) = open.project.sound_mut(id)
        {
            existing.file = info;
            existing.source = Some(source);
            open.missing.retain(|m| m != id);
            open.analysis_failed.retain(|m| m != id);
            return Ok(Some(id.clone()));
        }
        let mut sound = Sound::new(request.category, request.name.trim().to_string(), info);
        sound.source = Some(source);
        let sound_id = sound.id.clone();
        open.project.sounds.push(sound);
        Ok(Some(sound_id))
    });
    let cancelled = matches!(&outcome, Err(e) if e == "CANCELLED");
    match &outcome {
        Ok(_) => crate::logging::info(&format!("Downloaded \"{}\" from {}", request.name, request.url)),
        Err(_) if cancelled => crate::logging::info(&format!("Download cancelled: {}", request.url)),
        Err(e) => crate::logging::error(&format!("Download failed ({}): {e}", request.url)),
    }
    if let Some(job) = s.downloads.iter_mut().find(|j| j.id == id) {
        match &outcome {
            Ok(sound_id) => {
                job.status = JobStatus::Done;
                job.progress = 1.0;
                job.message = None;
                job.sound_id = sound_id.clone();
            }
            Err(_) if cancelled => job.status = JobStatus::Cancelled,
            Err(e) => {
                job.status = JobStatus::Failed;
                job.message = Some(if e == "TOOLS_MISSING" {
                    "yt-dlp and ffmpeg aren't installed".into()
                } else {
                    e.clone()
                });
            }
        }
    }
    if let Ok(Some(new_id)) = &outcome
        && request.video_height.is_some()
    {
        // Looked at next; offered for conversion if it may not play (an MP4-only site).
        crate::video_convert::note_new(&mut s, std::slice::from_ref(new_id), crate::video_convert::Origin::Added);
    }
    if let Ok(Some(_)) = &outcome {
        problems.push(format!("Downloaded \"{}\".", request.name.trim()));
        problems.extend(s.project_changed(Duration::ZERO));
    }
    save_unfinished(&s);
    emit_downloads(app, &s);
    emit_state(app, &s, problems);
}

/// `rename` only works within one disk; the cache may be on another, so fall back to copying.
fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|e| format!("could not save the download: {e}"))?;
    let _ = fs::remove_file(from);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_direct_audio_links() {
        assert_eq!(direct_extension("https://x.org/a/rain%20loop.MP3?dl=1").as_deref(), Some("mp3"));
        assert_eq!(direct_extension("https://www.youtube.com/watch?v=abc"), None);
        assert_eq!(direct_extension("https://x.org/page.html"), None);
        assert_eq!(host_of("https://www.freesound.org/s/1"), "freesound.org");
        assert_eq!(percent_decode("rain%20loop+2"), "rain loop 2");
    }

    #[test]
    fn reads_ffmpeg_progress() {
        // As seen from a real part-of-a-video download (decision 0019).
        let line = "size=     206KiB time=00:00:13.00 bitrate= 129.9kbits/s speed= 227x elapsed=0:00:00.05";
        assert_eq!(ffmpeg_time_ms(line), Some(13_000));
        assert_eq!(ffmpeg_time_ms("size= 1KiB time=01:02:03.45 bitrate=1"), Some(3_723_450));
        assert_eq!(ffmpeg_time_ms("size= 0KiB time=N/A bitrate=N/A"), None);
        assert_eq!(ffmpeg_time_ms("size= 0KiB time=-00:00:00.02 bitrate=N/A"), None);
        assert_eq!(ffmpeg_time_ms("[download] Destination: audio.m4a"), None);
    }

    #[test]
    fn output_is_split_at_carriage_returns_too() {
        let output = "[info] one\r\nframe time=00:00:01.00\rframe time=00:00:02.00\r[ExtractAudio] two\n";
        let lines: Vec<String> = output_lines(output.as_bytes()).collect();
        assert_eq!(lines, ["[info] one", "frame time=00:00:01.00", "frame time=00:00:02.00", "[ExtractAudio] two"]);
    }

    #[test]
    fn leftovers_keep_only_what_can_be_continued() {
        let tmp = crate::storage::test_support::TempDir::new();
        for name in ["audio.webm.part", "audio.webm.ytdl", "audio.m4a", "audio.temp.m4a"] {
            fs::write(tmp.0.join(name), b"x").unwrap();
        }
        clean_leftovers(&tmp.0, true);
        let mut left: Vec<String> =
            fs::read_dir(&tmp.0).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["audio.webm.part", "audio.webm.ytdl"]);
        // A part of a video can't be continued: everything goes.
        clean_leftovers(&tmp.0, false);
        assert_eq!(fs::read_dir(&tmp.0).unwrap().count(), 0);
    }

    #[test]
    fn saved_downloads_of_missing_projects_are_dropped() {
        let root = crate::storage::test_support::TempDir::new();
        let (data, cache, project) = (root.0.join("data"), root.0.join("cache"), root.0.join("Strahd"));
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join(storage::PROJECT_FILE), "{}").unwrap();
        for id in ["dl-kept", "dl-gone", "dl-stray"] {
            fs::create_dir_all(cache.join("downloads").join(id)).unwrap();
        }
        let request = |name: &str| DownloadRequest {
            url: "https://x.org/a.mp3".into(),
            kind: "direct".into(),
            title: name.into(),
            site: "x.org".into(),
            ext: "mp3".into(),
            category: Category::Effects,
            folder: String::new(),
            name: name.into(),
            range: None,
            replace: None,
            video_height: None,
        };
        let saved = vec![
            SavedDownload { id: "dl-kept".into(), project_dir: project.clone(), request: request("kept") },
            SavedDownload { id: "dl-gone".into(), project_dir: root.0.join("Deleted"), request: request("gone") },
        ];
        fs::write(data.join(SAVED_FILE), serde_json::to_string(&saved).unwrap()).unwrap();

        let loaded = load_saved(&data, &cache);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "dl-kept");
        assert!(cache.join("downloads/dl-kept").is_dir());
        assert!(!cache.join("downloads/dl-gone").exists());
        assert!(!cache.join("downloads/dl-stray").exists());
    }

    #[test]
    fn picks_the_best_video_format_for_each_resolution_limit() {
        // Like a YouTube page: H.264 MP4s and VP9 WebMs, picture only, plus an audio format.
        let formats: Vec<serde_json::Value> = serde_json::from_str(
            r#"[
              {"vcodec":"avc1.640028","acodec":"none","height":1080,"ext":"mp4","filesize":900},
              {"vcodec":"vp09.00.40.08","acodec":"none","height":1080,"ext":"webm","filesize":700},
              {"vcodec":"vp09.00.31.08","acodec":"none","height":720,"ext":"webm","filesize":400},
              {"vcodec":"avc1.64002a","acodec":"none","height":2160,"ext":"mp4","tbr":8000},
              {"vcodec":"none","acodec":"mp4a.40.2","ext":"m4a"}
            ]"#,
        )
        .unwrap();
        let options = video_options(&formats, Some(10_000));
        let at = |limit: u32| options.iter().find(|o| o.max_height == limit).unwrap();
        assert_eq!((at(720).height, at(720).codec.as_str()), (720, "vp9"));
        // WebM VP9 is preferred over an MP4 of the same height.
        assert_eq!((at(1080).ext.as_str(), at(1080).size), ("webm", Some(700)));
        assert!(at(1080).plays_everywhere);
        // "Best": a playable 1080p WebM beats a 4K MP4 that would need converting.
        assert_eq!(at(0).height, 1080);
        assert_eq!(short_codec("av01.0.08M.08"), "av1");
        assert_eq!(direct_video_extension("https://x.org/a/gate.WEBM?dl=1").as_deref(), Some("webm"));
        assert_eq!(direct_video_extension("https://x.org/rain.mp3"), None);
    }

    #[test]
    fn rejects_things_that_are_not_urls() {
        assert!(probe(Path::new("/nowhere"), "not a url").is_err());
    }
}

#[cfg(test)]
mod online {
    /// `COMILITO_TOOLS=<dir with yt-dlp> cargo test probes_real_urls -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn probes_real_urls() {
        let tools = std::path::PathBuf::from(std::env::var("COMILITO_TOOLS").expect("set COMILITO_TOOLS"));
        let page = super::probe(&tools, "https://www.youtube.com/watch?v=jNQXAC9IVRw").unwrap();
        println!("{page:?}");
        assert_eq!(page.kind, "page");
        assert!(page.duration_ms.unwrap() > 10_000);
        let direct = super::probe(&tools, "https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg").unwrap();
        println!("{direct:?}");
        assert_eq!(direct.kind, "direct");
        assert_eq!(direct.ext, "ogg");
    }
}
