//! The external programs Comilito downloads and manages itself (decision 0019):
//!
//! - **yt-dlp** fetches audio from video sites. Standalone builds from its GitHub releases.
//! - **ffmpeg** (with **ffprobe**) converts and cuts audio, for yt-dlp and for cropping
//!   (decision 0020), and converts videos (decision 0035). ffmpeg.org publishes no binaries,
//!   so we use BtbN's release builds (`BtbN/FFmpeg-Builds`, Linux and Windows; decision 0051),
//!   from the branch in `FFMPEG_BRANCH`.
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
const FFMPEG_RELEASE: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/";
/// The ffmpeg release branch to install. Not simply the newest: each branch is built with a
/// version of NVIDIA's encoder headers, and a newer one refuses older drivers. In November 2026
/// the 9.0 builds (and the nightly ones) need driver 610, while 8.1 works from much older
/// drivers on, and with newer ones too. Raise it once drivers have caught up. If BtbN stops
/// publishing this branch, the newest release is installed instead.
const FFMPEG_BRANCH: &str = "8.1";
/// Written next to ffmpeg: the name of the release file it came from. Older Comilito versions
/// installed yt-dlp's nightly build and wrote nothing, which is how that build is recognised.
const FFMPEG_BUILD_FILE: &str = "build.txt";

/// What the UI shows about the tools.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    /// yt-dlp's version, if installed.
    pub yt_dlp: Option<String>,
    pub ffmpeg: bool,
    /// The installed ffmpeg's release ("9.0"), or `None` for one installed before decision
    /// 0051 (yt-dlp's nightly build, which GPU encoding may not work with).
    pub ffmpeg_version: Option<String>,
    /// After an install or update: what was downloaded ("yt-dlp", "ffmpeg"); empty when
    /// everything was up to date.
    pub updated: Vec<String>,
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

/// ffprobe, next to ffmpeg (it looks at video files, decision 0035).
pub fn ffprobe_path(dir: &Path) -> Option<PathBuf> {
    let ffmpeg = ffmpeg_path(dir)?;
    let ours = ffmpeg.with_file_name(exe("ffprobe"));
    if ours.is_file() {
        return Some(ours);
    }
    // ffmpeg found on the PATH (macOS): ffprobe usually comes with it.
    (ffmpeg == Path::new("ffmpeg")).then(|| PathBuf::from("ffprobe"))
}

pub fn status(dir: &Path) -> ToolsStatus {
    let yt = yt_dlp_path(dir);
    let yt_dlp = yt.is_file().then(|| version_of(&yt)).flatten();
    ToolsStatus {
        yt_dlp,
        ffmpeg: ffmpeg_path(dir).is_some(),
        ffmpeg_version: installed_ffmpeg_build(dir).as_deref().and_then(release_version),
        updated: Vec::new(),
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

/// Makes a command start in a process group of its own (Unix), so `kill_tree` can later stop it
/// together with the programs it starts (yt-dlp starts ffmpeg). On Windows `kill_tree` finds
/// those through the process tree instead.
pub fn own_process_group(cmd: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // 0 means "a new group, numbered after the child's own process id".
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = cmd;
}

/// Stops a process and everything it started, then waits for it. `Child::kill` alone stops
/// only the process itself: an ffmpeg started by yt-dlp would keep running.
pub fn kill_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        // A negative id sends the signal to the whole group (needs `own_process_group`).
        // `unsafe` because it's a raw C call; it touches no Rust memory, so nothing can go wrong
        // on our side.
        let group = child.id() as libc::pid_t;
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
    #[cfg(windows)]
    {
        let _ = hidden_command(Path::new("taskkill"))
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .output();
    }
    // In case the above failed (or on other systems): at least the process itself.
    let _ = child.kill();
    let _ = child.wait();
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
    /// The platform in BtbN's ffmpeg file names ("linux64"); the file itself is picked from the
    /// release's checksum list (`ffmpeg_release`), as its name holds the version.
    ffmpeg: Option<&'static str>,
}

fn assets() -> Result<Assets, String> {
    // `cfg!(...)` is the expression form of `#[cfg]`: a compile-time true/false.
    let arm = cfg!(target_arch = "aarch64");
    if cfg!(target_os = "linux") {
        Ok(Assets {
            yt_dlp: if arm { "yt-dlp_linux_aarch64" } else { "yt-dlp_linux" },
            ffmpeg: Some(if arm { "linuxarm64" } else { "linux64" }),
        })
    } else if cfg!(windows) {
        Ok(Assets {
            yt_dlp: if arm { "yt-dlp_arm64.exe" } else { "yt-dlp.exe" },
            ffmpeg: Some(if arm { "winarm64" } else { "win64" }),
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

/// Downloads a published checksum list ("<sha256>  <file name>" per line).
fn checksum_list(sums_url: &str) -> Result<String, String> {
    ureq::get(sums_url)
        .call()
        .map_err(|e| format!("could not check the download: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("could not check the download: {e}"))
}

/// `(sha256, file name)` for each line of a checksum list.
fn checksum_entries(list: &str) -> impl Iterator<Item = (String, &str)> {
    list.lines()
        .filter_map(|line| line.split_once(char::is_whitespace))
        .map(|(hash, name)| (hash.to_lowercase(), name.trim().trim_start_matches('*')))
}

/// Finds `file` in a published checksum list.
fn expected_sha256(sums_url: &str, file: &str) -> Result<String, String> {
    let list = checksum_list(sums_url)?;
    checksum_entries(&list)
        .find(|(_, name)| *name == file)
        .map(|(hash, _)| hash)
        .ok_or_else(|| format!("{file} is not listed in the published checksums"))
}

/// The version in a BtbN release file name: "ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz" → "9.0".
fn release_version(file: &str) -> Option<String> {
    let version = file.strip_prefix("ffmpeg-n")?.split_once("-latest-")?.0;
    let valid = !version.is_empty() && version.split('.').all(|part| part.parse::<u32>().is_ok());
    valid.then(|| version.to_string())
}

/// The release build to install for `platform` from BtbN's checksum list, with its checksum:
/// the `branch` one if it's published, else the newest. Only the full GPL builds in one file
/// ("-gpl-8.1.tar.xz"), not "-lgpl-" or "-shared-", and not the nightly "master" builds.
/// Versions compare as numbers, so 10.0 beats 9.1.
fn ffmpeg_release(list: &str, platform: &str, branch: &str) -> Option<(String, String)> {
    let builds: Vec<(String, String, String)> = checksum_entries(list)
        .filter_map(|(hash, name)| {
            let version = release_version(name)?;
            let expected = format!("ffmpeg-n{version}-latest-{platform}-gpl-{version}.");
            let ext_ok = name.ends_with(".tar.xz") || name.ends_with(".zip");
            (name.starts_with(&expected) && ext_ok).then(|| (version, hash, name.to_string()))
        })
        .collect();
    let pinned = builds.iter().find(|(version, _, _)| version == branch);
    pinned
        .or_else(|| {
            builds
                .iter()
                .max_by_key(|(version, _, _)| version.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>())
        })
        .map(|(_, hash, name)| (name.clone(), hash.clone()))
}

/// The release file the installed ffmpeg came from, if it was installed by this version.
pub fn installed_ffmpeg_build(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(ffmpeg_dir(dir).join(FFMPEG_BUILD_FILE)).ok()?;
    Some(text.trim().to_string()).filter(|t| !t.is_empty())
}

/// Whether ffmpeg should be (re)installed: missing, from before decision 0051, or not the
/// release build `install_ffmpeg` would pick now (a new build of the branch, or a new branch). Asks the internet only in the last case; offline, it's left alone.
pub fn ffmpeg_needs_install(dir: &Path) -> bool {
    if ffmpeg_dir(dir).join(exe("ffmpeg")).is_file() {
        let Some(installed) = installed_ffmpeg_build(dir) else { return true };
        let Some(platform) = assets().ok().and_then(|a| a.ffmpeg) else { return false };
        checksum_list(&format!("{FFMPEG_RELEASE}checksums.sha256"))
            .ok()
            .and_then(|list| ffmpeg_release(&list, platform, FFMPEG_BRANCH))
            .is_some_and(|(newest, _)| newest != installed)
    } else {
        // Not ours: on macOS one on the PATH may be used, and there's nothing to install.
        assets().is_ok_and(|a| a.ffmpeg.is_some())
    }
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

/// Installs (or replaces) ffmpeg and ffprobe with the BtbN release build (`FFMPEG_BRANCH`).
pub fn install_ffmpeg(dir: &Path, progress: &mut dyn FnMut(InstallProgress)) -> Result<(), String> {
    let Some(platform) = assets()?.ffmpeg else {
        return Err("on macOS, install ffmpeg yourself (for example with Homebrew) for now".into());
    };
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let list = checksum_list(&format!("{FFMPEG_RELEASE}checksums.sha256"))?;
    let (asset, expected) =
        ffmpeg_release(&list, platform, FFMPEG_BRANCH).ok_or("no ffmpeg release build is published for this computer")?;
    let archive = dir.join(&asset);
    let actual = download(&format!("{FFMPEG_RELEASE}{asset}"), &archive, "ffmpeg", progress)?;
    let result = verify(&actual, &expected, "ffmpeg").and_then(|_| {
        progress(InstallProgress { step: "unpacking ffmpeg".into(), done: 0, total: 0 });
        unpack_ffmpeg(&archive, dir)
    });
    let _ = fs::remove_file(&archive);
    result?;
    fs::write(ffmpeg_dir(dir).join(FFMPEG_BUILD_FILE), &asset).map_err(|e| e.to_string())
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
    fn picks_the_pinned_branch_else_the_newest_full_release_build() {
        let list = "\
aa  ffmpeg-master-latest-linux64-gpl.tar.xz
bb  ffmpeg-n8.1-latest-linux64-gpl-8.1.tar.xz
cc  ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz
dd  ffmpeg-n8.1-latest-linux64-lgpl-8.1.tar.xz
ee  ffmpeg-n8.1-latest-linux64-gpl-shared-8.1.tar.xz
ff  ffmpeg-n10.0-latest-linuxarm64-gpl-10.0.tar.xz
11  ffmpeg-n9.1-latest-linuxarm64-gpl-9.1.tar.xz
00  ffmpeg-n8.1-latest-win64-gpl-8.1.zip
";
        let linux = ffmpeg_release(list, "linux64", "8.1").unwrap();
        assert_eq!(linux, ("ffmpeg-n8.1-latest-linux64-gpl-8.1.tar.xz".to_string(), "bb".to_string()));
        assert_eq!(ffmpeg_release(list, "win64", "8.1").unwrap().0, "ffmpeg-n8.1-latest-win64-gpl-8.1.zip");
        // 8.1 not published for ARM here: the newest, compared as numbers.
        assert_eq!(ffmpeg_release(list, "linuxarm64", "8.1").unwrap().0, "ffmpeg-n10.0-latest-linuxarm64-gpl-10.0.tar.xz");
        assert_eq!(ffmpeg_release(list, "winarm64", "8.1"), None);
        assert_eq!(release_version("ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz").as_deref(), Some("9.0"));
        assert_eq!(release_version("ffmpeg-master-latest-linux64-gpl.tar.xz"), None);
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
        assert!(st.ffmpeg_version.is_some(), "a release build, with its version recorded");
        assert!(!ffmpeg_needs_install(&dir.0));
        assert_eq!(yt_dlp_update_available(&dir.0), Some(false));
    }

    /// Like yt-dlp starting ffmpeg: a shell starts `sleep`, and both must stop.
    #[cfg(unix)]
    #[test]
    fn kill_tree_stops_the_programs_a_process_started() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 30 & echo $!; wait"]).stdout(std::process::Stdio::piped());
        own_process_group(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(child.stdout.take().unwrap()), &mut line).unwrap();
        let grandchild: libc::pid_t = line.trim().parse().unwrap();

        kill_tree(&mut child);
        // Signal 0 only checks whether the process exists. A killed one may linger a moment
        // as a "zombie" until init reaps it, so allow it a little time.
        let gone = (0..50).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            (unsafe { libc::kill(grandchild, 0) }) != 0
                || std::fs::read_to_string(format!("/proc/{grandchild}/stat")).is_ok_and(|s| s.contains(") Z "))
        });
        assert!(gone, "the grandchild is still running");
    }
}
