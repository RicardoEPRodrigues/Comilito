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
    project::{Category, Sound, Source, TimeRange},
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
        .args(["--no-playlist", "--newline", "--no-warnings", "--no-mtime", "--color", "never"])
        .args(["-f", "bestaudio[ext=m4a]/bestaudio/best"])
        .args(["-x", "--audio-format", "m4a", "--audio-quality", "192K"])
        .arg("--ffmpeg-location")
        .arg(tools::ffmpeg_dir(&tools_dir))
        .args(["--progress-template", "download:COMILITO %(progress._percent_str)s"])
        .arg("-o")
        .arg(tmp.join("audio.%(ext)s"));
    if let Some(r) = request.range {
        // A part of a video is fetched by ffmpeg, which reports no percentage: say so instead
        // of showing a progress bar stuck at 0%.
        update(app, id, |j| j.message = Some("downloading the selected part…".into()));
        cmd.arg("--download-sections")
            .arg(format!("*{:.3}-{:.3}", r.start_ms as f64 / 1000.0, r.end_ms as f64 / 1000.0));
    }
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
    let result = watch_yt_dlp(app, id, &tmp, &child, stdout, stderr);
    if let Ok(mut s) = app.state::<Mutex<AppState>>().lock() {
        s.download_process = None;
    }
    result
}

/// Follows a running yt-dlp until it ends (or the download is cancelled) and returns the
/// audio file it left.
fn watch_yt_dlp(
    app: &AppHandle,
    id: &str,
    tmp: &Path,
    child: &Mutex<Child>,
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
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if let Some(rest) = line.trim().strip_prefix("COMILITO") {
                    // "N/A" (unknown size) is skipped rather than shown as 0%.
                    let Ok(percent) = rest.trim().trim_end_matches('%').trim().parse::<f32>() else {
                        continue;
                    };
                    if last.elapsed() > Duration::from_millis(250) || percent >= 100.0 {
                        last = Instant::now();
                        update(&app2, &id2, |j| j.progress = (percent / 100.0).clamp(0.0, 1.0));
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
            let _ = process.kill();
            let _ = process.wait();
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
    // yt-dlp chose the final name's extension; find the audio it left.
    fs::read_dir(tmp)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.extension()
                .is_some_and(|e| storage::AUDIO_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
        })
        .ok_or_else(|| "yt-dlp finished but left no audio file".to_string())
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
        let folder = storage::category_folder_path(dir, request.category, &request.folder);
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
