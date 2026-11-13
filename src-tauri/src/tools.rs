//! The external programs Comilito downloads and manages itself (decision 0019):
//!
//! - **yt-dlp** fetches audio from video sites. Standalone builds from its GitHub releases.
//! - **ffmpeg** (with **ffprobe**) converts and cuts audio, for yt-dlp and for cropping
//!   (decision 0020). ffmpeg.org publishes no binaries, so we use the builds yt-dlp
//!   recommends (`yt-dlp/FFmpeg-Builds`, Linux and Windows).
//!
//! Both are installed into `<app data>/tools/` the first time they're needed. Every download
//! is checked against the SHA-256 checksums published with the release before it's used.
//! We never use a copy installed elsewhere on the system, so the version is always known.
//! (macOS is the exception for ffmpeg until a source is chosen: there, one on the PATH is used.)

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;
use sha2::{Digest, Sha256};

const YT_DLP_RELEASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/";
const FFMPEG_RELEASE: &str = "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/";

/// What the UI shows about the tools.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    /// yt-dlp's version, if installed.
    pub yt_dlp: Option<String>,
    pub ffmpeg: bool,
    /// Disk space used by the tools folder, in bytes.
    pub size: u64,
}

/// Progress of an installation, sent to the UI as `tools-progress` events.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub step: String,
    pub done: u64,
    pub total: u64,
}

pub fn tools_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tools")
}

/// `name` with `.exe` on Windows.
fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

pub fn yt_dlp_path(dir: &Path) -> PathBuf {
    dir.join(exe("yt-dlp"))
}

/// The folder holding ffmpeg and ffprobe (what yt-dlp's `--ffmpeg-location` wants).
pub fn ffmpeg_dir(dir: &Path) -> PathBuf {
    dir.join("ffmpeg")
}

/// The ffmpeg program to run, if there is one.
pub fn ffmpeg_path(dir: &Path) -> Option<PathBuf> {
    let ours = ffmpeg_dir(dir).join(exe("ffmpeg"));
    if ours.is_file() {
        return Some(ours);
    }
    // macOS: no managed build yet, so use one on the PATH if the user has it.
    if cfg!(target_os = "macos") && Command::new("ffmpeg").arg("-version").output().is_ok() {
        return Some(PathBuf::from("ffmpeg"));
    }
    None
}

pub fn status(dir: &Path) -> ToolsStatus {
    let yt = yt_dlp_path(dir);
    let yt_dlp = yt.is_file().then(|| version_of(&yt)).flatten();
    ToolsStatus {
        yt_dlp,
        ffmpeg: ffmpeg_path(dir).is_some(),
        size: folder_size(dir),
    }
}

fn version_of(program: &Path) -> Option<String> {
    let out = hidden_command(program).arg("--version").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A `Command` that doesn't flash a console window on Windows.
pub fn hidden_command(program: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

fn folder_size(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else { return 0 };
    entries
        .flatten()
        .map(|e| {
            let path = e.path();
            if path.is_dir() {
                folder_size(&path)
            } else {
                e.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

/// The release file names for this computer, or an error on unsupported platforms.
struct Assets {
    yt_dlp: &'static str,
    ffmpeg: Option<&'static str>,
}

fn assets() -> Result<Assets, String> {
    // `cfg!(...)` is the expression form of `#[cfg]`: a compile-time true/false.
    let arm = cfg!(target_arch = "aarch64");
    if cfg!(target_os = "linux") {
        Ok(Assets {
            yt_dlp: if arm { "yt-dlp_linux_aarch64" } else { "yt-dlp_linux" },
            ffmpeg: Some(if arm {
                "ffmpeg-master-latest-linuxarm64-gpl.tar.xz"
            } else {
                "ffmpeg-master-latest-linux64-gpl.tar.xz"
            }),
        })
    } else if cfg!(windows) {
        Ok(Assets {
            yt_dlp: if arm { "yt-dlp_arm64.exe" } else { "yt-dlp.exe" },
            ffmpeg: Some(if arm {
                "ffmpeg-master-latest-winarm64-gpl.zip"
            } else {
                "ffmpeg-master-latest-win64-gpl.zip"
            }),
        })
    } else if cfg!(target_os = "macos") {
        Ok(Assets {
            yt_dlp: "yt-dlp_macos",
            ffmpeg: None,
        })
    } else {
        Err("downloading audio isn't supported on this system yet".into())
    }
}

/// Reads a published checksum list ("<sha256>  <file name>" per line) and finds `file`.
fn expected_sha256(sums_url: &str, file: &str) -> Result<String, String> {
    let text = ureq::get(sums_url)
        .call()
        .map_err(|e| format!("could not check the download: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("could not check the download: {e}"))?;
    text.lines()
        .filter_map(|line| line.split_once(char::is_whitespace))
        .find(|(_, name)| name.trim().trim_start_matches('*') == file)
        .map(|(hash, _)| hash.to_lowercase())
        .ok_or_else(|| format!("{file} is not listed in the published checksums"))
}

/// Downloads `url` to `dest`, reporting progress, and returns the file's SHA-256.
fn download(url: &str, dest: &Path, step: &str, progress: &mut dyn FnMut(InstallProgress)) -> Result<String, String> {
    let mut response = ureq::get(url).call().map_err(|e| format!("could not download {url}: {e}"))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut reader = response.body_mut().with_config().limit(u64::MAX).reader();
    let mut file = fs::File::create(dest).map_err(|e| format!("could not write {}: {e}", dest.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    let mut last_report = 0u64;
    loop {
        let n = reader.read(&mut buffer).map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| format!("could not write {}: {e}", dest.display()))?;
        hasher.update(&buffer[..n]);
        done += n as u64;
        // Report every ~512 KB, not for every chunk.
        if done - last_report > 512 * 1024 {
            last_report = done;
            progress(InstallProgress { step: step.into(), done, total });
        }
    }
    progress(InstallProgress { step: step.into(), done, total });
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn verify(actual: &str, expected: &str, what: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what} didn't match its published checksum, so it wasn't installed"))
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Installs (or updates) yt-dlp.
pub fn install_yt_dlp(dir: &Path, progress: &mut dyn FnMut(InstallProgress)) -> Result<(), String> {
    let assets = assets()?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let expected = expected_sha256(&format!("{YT_DLP_RELEASE}SHA2-256SUMS"), assets.yt_dlp)?;
    let tmp = dir.join(format!("{}.download", exe("yt-dlp")));
    let actual = download(&format!("{YT_DLP_RELEASE}{}", assets.yt_dlp), &tmp, "yt-dlp", progress)?;
    if let Err(e) = verify(&actual, &expected, "yt-dlp") {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    make_executable(&tmp)?;
    fs::rename(&tmp, yt_dlp_path(dir)).map_err(|e| e.to_string())
}

/// Installs ffmpeg and ffprobe from the yt-dlp FFmpeg build.
pub fn install_ffmpeg(dir: &Path, progress: &mut dyn FnMut(InstallProgress)) -> Result<(), String> {
    let Some(asset) = assets()?.ffmpeg else {
        return Err("on macOS, install ffmpeg yourself (for example with Homebrew) for now".into());
    };
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let expected = expected_sha256(&format!("{FFMPEG_RELEASE}checksums.sha256"), asset)?;
    let archive = dir.join(asset);
    let actual = download(&format!("{FFMPEG_RELEASE}{asset}"), &archive, "ffmpeg", progress)?;
    let result = verify(&actual, &expected, "ffmpeg").and_then(|_| {
        progress(InstallProgress { step: "unpacking ffmpeg".into(), done: 0, total: 0 });
        unpack_ffmpeg(&archive, dir)
    });
    let _ = fs::remove_file(&archive);
    result
}

/// Takes `ffmpeg` and `ffprobe` out of the downloaded archive into `tools/ffmpeg/`.
fn unpack_ffmpeg(archive: &Path, dir: &Path) -> Result<(), String> {
    let staging = dir.join("ffmpeg-unpack");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;

    if archive.extension().is_some_and(|e| e == "zip") {
        let file = fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("could not open the ffmpeg archive: {e}"))?;
        zip.extract(&staging).map_err(|e| format!("could not unpack ffmpeg: {e}"))?;
    } else {
        // .tar.xz: every Linux system has `tar` with xz support, so no extra crate is needed.
        let status = Command::new("tar")
            .arg("-xJf")
            .arg(archive)
            .arg("-C")
            .arg(&staging)
            .status()
            .map_err(|e| format!("could not run tar to unpack ffmpeg: {e}"))?;
        if !status.success() {
            return Err("could not unpack ffmpeg (is xz installed?)".into());
        }
    }

    let target = ffmpeg_dir(dir);
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    for program in ["ffmpeg", "ffprobe"] {
        let name = exe(program);
        let found = find_file(&staging, &name).ok_or_else(|| format!("{name} wasn't in the ffmpeg archive"))?;
        let dest = target.join(&name);
        fs::copy(&found, &dest).map_err(|e| e.to_string())?;
        make_executable(&dest)?;
    }
    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    let mut to_visit = vec![root.to_path_buf()];
    while let Some(dir) = to_visit.pop() {
        for entry in fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                to_visit.push(path);
            } else if entry.file_name() == name {
                return Some(path);
            }
        }
    }
    None
}

/// Whether a newer yt-dlp is published: compares the installed file's hash with the latest
/// release's checksum. `None` if not installed or the check failed (offline...).
pub fn yt_dlp_update_available(dir: &Path) -> Option<bool> {
    let path = yt_dlp_path(dir);
    if !path.is_file() {
        return None;
    }
    let expected = expected_sha256(&format!("{YT_DLP_RELEASE}SHA2-256SUMS"), assets().ok()?.yt_dlp).ok()?;
    let installed = crate::bundle::sha256_file(&path).ok()?;
    Some(installed != expected)
}

/// Removes the downloaded tools (Preferences › Projects & storage).
pub fn remove_all(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| format!("could not remove the tools: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_platform_has_release_assets() {
        // Linux and Windows have both tools; this test runs where the app is developed.
        let a = assets().unwrap();
        assert!(a.yt_dlp.starts_with("yt-dlp"));
        if cfg!(any(target_os = "linux", windows)) {
            assert!(a.ffmpeg.is_some());
        }
    }

    #[test]
    fn checksum_mismatch_is_refused() {
        assert!(verify("aa", "bb", "yt-dlp").is_err());
        assert!(verify("aa", "aa", "yt-dlp").is_ok());
    }

    /// Downloads the real tools. Slow and needs the internet, so only on request:
    /// `cargo test install_real_tools -- --ignored`
    #[test]
    #[ignore]
    fn install_real_tools() {
        let dir = crate::storage::test_support::TempDir::new();
        let mut log = |p: InstallProgress| println!("{} {}/{}", p.step, p.done, p.total);
        install_yt_dlp(&dir.0, &mut log).unwrap();
        install_ffmpeg(&dir.0, &mut log).unwrap();
        let st = status(&dir.0);
        println!("{st:?}");
        assert!(st.yt_dlp.is_some());
        assert!(st.ffmpeg);
        assert_eq!(yt_dlp_update_available(&dir.0), Some(false));
    }
}
