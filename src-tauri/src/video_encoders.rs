//! Converting videos on the GPU (decision 0051).
//!
//! Videos are converted to WebM so every webview plays them (decision 0035). On the CPU that's
//! VP9 (libvpx). Most GPUs that can encode a WebM codec at all encode AV1, which webviews play
//! too: NVIDIA (NVENC, RTX 40 and newer), Intel Arc and AMD RDNA 3 (VA-API on Linux, QSV and AMF
//! on Windows); older Intel GPUs encode VP9 through VA-API.
//!
//! Which of those work depends on the GPU, its driver and the ffmpeg build, so each candidate is
//! tried once with a tiny test encode (`detect`), and only the ones that work are offered. The
//! result is kept for the session (`AppState::video_encoders`). Preferences › Video picks one,
//! or "auto" (the first that works) or "cpu".
//!
//! The Vulkan encoders (`av1_vulkan`) aren't used: tested on an RTX 5070 Ti, their quality was
//! far below libvpx's at three times the file size.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::tools;

/// How an encoder is driven. Not sent to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// NVIDIA's AV1 encoder on GPU number `gpu` (as `nvidia-smi` counts them).
    Nvenc { gpu: u32 },
    /// VA-API (Linux, Intel and AMD) on a render node, with `codec` "av1" or "vp9".
    Vaapi { device: PathBuf, codec: &'static str },
    /// Intel Quick Sync (Windows).
    Qsv,
    /// AMD AMF (Windows).
    Amf,
}

/// A GPU encoder that passed its test.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Encoder {
    /// Stable for this computer, stored in the preferences ("nvenc:0", "vaapi:renderD128:av1").
    pub id: String,
    /// For the Preferences list: "NVIDIA GeForce RTX 5070 Ti · AV1 (NVENC)".
    pub label: String,
    #[serde(skip)]
    pub kind: Kind,
}

impl Encoder {
    /// ffmpeg arguments that go before the input (`-i`).
    pub fn input_args(&self) -> Vec<String> {
        match &self.kind {
            Kind::Vaapi { device, .. } => vec!["-vaapi_device".into(), device.to_string_lossy().into_owned()],
            _ => Vec::new(),
        }
    }

    /// ffmpeg arguments for the video stream, for a video of `width` × `height` (0 if unknown).
    pub fn output_args(&self, width: u32, height: u32) -> Vec<String> {
        // NVENC has a constant-quality mode; 36 matched libvpx-vp9 at crf 32 on real footage
        // (same SSIM, a similar size). The others get a bitrate for the picture size instead:
        // their quality modes differ by driver and couldn't be tested here.
        let pixels = if width > 0 && height > 0 { width as u64 * height as u64 } else { 1920 * 1080 };
        let rate = |factor: f64| ((pixels as f64 * factor) as u64).max(500_000);
        let vbr = |codec: &str, bits: u64| {
            let mut v = vec!["-c:v".to_string(), codec.to_string()];
            v.extend(["-b:v".into(), bits.to_string(), "-maxrate".into(), (bits * 2).to_string()]);
            v
        };
        let s = |list: &[&str]| list.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        match &self.kind {
            Kind::Nvenc { gpu } => {
                let mut v = s(&["-c:v", "av1_nvenc", "-gpu"]);
                v.push(gpu.to_string());
                v.extend(s(&["-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq", "36", "-b:v", "0", "-pix_fmt", "yuv420p"]));
                v
            }
            // About 2.5 Mbit/s for 1080p in AV1; VP9 needs a little more for the same look.
            Kind::Vaapi { codec, .. } => {
                let mut v = s(&["-vf", "format=nv12,hwupload", "-rc_mode", "VBR"]);
                let bits = if *codec == "av1" { rate(1.2) } else { rate(1.7) };
                v.extend(vbr(if *codec == "av1" { "av1_vaapi" } else { "vp9_vaapi" }, bits));
                v
            }
            Kind::Qsv => {
                let mut v = vbr("av1_qsv", rate(1.2));
                v.extend(s(&["-pix_fmt", "nv12"]));
                v
            }
            Kind::Amf => {
                let mut v = vbr("av1_amf", rate(1.2));
                v.extend(s(&["-rc", "vbr_peak", "-pix_fmt", "nv12"]));
                v
            }
        }
    }
}

/// The CPU encoder (libvpx VP9): what's used without a GPU, and when a GPU conversion fails.
/// Constant quality (crf); "good" with cpu-used 4 is a fair speed for the quality, and row-mt
/// uses every core.
pub fn cpu_args() -> Vec<String> {
    ["-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0", "-deadline", "good", "-cpu-used", "4", "-row-mt", "1", "-pix_fmt", "yuv420p"]
        .iter()
        .map(|a| a.to_string())
        .collect()
}

/// The encoder to use for `preference` ("auto", "cpu" or an encoder's id), or `None` for the
/// CPU. A chosen encoder that's no longer there (another GPU, a new driver) counts as "auto".
pub fn choose<'a>(preference: &str, encoders: &'a [Encoder]) -> Option<&'a Encoder> {
    match preference {
        "cpu" => None,
        id => encoders.iter().find(|e| e.id == id).or_else(|| encoders.first()),
    }
}

/// Finds the GPU encoders that work here, best first. Takes a few seconds (one test encode
/// per candidate), so it runs on a worker thread, never with the state locked.
pub fn detect(ffmpeg: &Path) -> Vec<Encoder> {
    let available = encoder_names(ffmpeg);
    let has = |name: &str| available.iter().any(|n| n == name);
    let mut candidates = Vec::new();

    if has("av1_nvenc") {
        let names = nvidia_gpu_names();
        if names.is_empty() {
            // No nvidia-smi: try the first GPU; a missing one just fails its test.
            candidates.push(nvenc(0, "NVIDIA GPU".into()));
        } else {
            for (i, name) in names.into_iter().enumerate() {
                candidates.push(nvenc(i as u32, name));
            }
        }
    }
    if cfg!(target_os = "linux") {
        for (device, name) in vaapi_devices() {
            // One encoder per device: AV1 if it can, else VP9 (older Intel).
            for (codec, encoder) in [("av1", "av1_vaapi"), ("vp9", "vp9_vaapi")] {
                if has(encoder) {
                    let node = device.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    candidates.push(Encoder {
                        id: format!("vaapi:{node}:{codec}"),
                        label: format!("{name} · {} (VA-API)", codec.to_uppercase()),
                        kind: Kind::Vaapi { device: device.clone(), codec },
                    });
                }
            }
        }
    }
    if cfg!(windows) {
        if has("av1_qsv") {
            candidates.push(Encoder { id: "qsv".into(), label: "Intel GPU · AV1 (Quick Sync)".into(), kind: Kind::Qsv });
        }
        if has("av1_amf") {
            candidates.push(Encoder { id: "amf".into(), label: "AMD GPU · AV1 (AMF)".into(), kind: Kind::Amf });
        }
    }

    let mut working: Vec<Encoder> = Vec::new();
    for candidate in candidates {
        // A VA-API device that already has a working AV1 encoder doesn't need VP9 too.
        if let Kind::Vaapi { device, codec: "vp9" } = &candidate.kind {
            if working.iter().any(|e| matches!(&e.kind, Kind::Vaapi { device: d, .. } if d == device)) {
                continue;
            }
        }
        if test_encode(ffmpeg, &candidate) {
            working.push(candidate);
        }
    }
    working
}

fn nvenc(gpu: u32, name: String) -> Encoder {
    Encoder { id: format!("nvenc:{gpu}"), label: format!("{name} · AV1 (NVENC)"), kind: Kind::Nvenc { gpu } }
}

/// The video encoders this ffmpeg was built with (`ffmpeg -encoders`).
fn encoder_names(ffmpeg: &Path) -> Vec<String> {
    let Ok(out) = tools::hidden_command(ffmpeg).args(["-hide_banner", "-encoders"]).output() else {
        return Vec::new();
    };
    // Lines look like " V....D av1_nvenc   NVIDIA NVENC av1 encoder (codec av1)".
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| line.trim_start().starts_with('V'))
        .filter_map(|line| line.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

/// NVIDIA GPU names in the driver's order ("NVIDIA GeForce RTX 5070 Ti"), if nvidia-smi is there.
fn nvidia_gpu_names() -> Vec<String> {
    let Ok(out) = tools::hidden_command(Path::new("nvidia-smi"))
        .args(["--query-gpu=name", "--format=csv,noheader"])
        .output()
    else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
}

/// Linux render nodes that may encode through VA-API, with a name for each. NVIDIA's are left
/// out: its VA-API driver only decodes (NVENC covers its GPUs).
fn vaapi_devices() -> Vec<(PathBuf, String)> {
    let Ok(entries) = std::fs::read_dir("/dev/dri") else { return Vec::new() };
    let mut nodes: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("renderD")))
        .collect();
    nodes.sort();
    nodes
        .into_iter()
        .filter_map(|node| {
            let name = node.file_name()?.to_string_lossy().into_owned();
            let vendor = std::fs::read_to_string(format!("/sys/class/drm/{name}/device/vendor")).unwrap_or_default();
            let maker = match vendor.trim() {
                "0x8086" => "Intel GPU",
                "0x1002" => "AMD GPU",
                "0x10de" => return None,
                _ => "GPU",
            };
            Some((node, format!("{maker} ({name})")))
        })
        .collect()
}

/// Encodes a fraction of a second of a test picture with `encoder`, to the null output.
/// Gives up after 20 s, in case a driver hangs.
fn test_encode(ffmpeg: &Path, encoder: &Encoder) -> bool {
    let mut cmd = tools::hidden_command(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error"])
        .args(encoder.input_args())
        .args(["-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30", "-frames:v", "10"])
        .args(encoder.output_args(640, 360))
        .args(["-f", "null", "-"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let Ok(mut child) = cmd.spawn() else { return false };
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nv(gpu: u32) -> Encoder {
        nvenc(gpu, format!("GPU {gpu}"))
    }

    #[test]
    fn choosing_an_encoder() {
        let found = [nv(0), nv(1)];
        assert_eq!(choose("cpu", &found), None);
        assert_eq!(choose("auto", &found).map(|e| e.id.as_str()), Some("nvenc:0"));
        assert_eq!(choose("nvenc:1", &found).map(|e| e.id.as_str()), Some("nvenc:1"));
        // A GPU that's gone: the first that works instead.
        assert_eq!(choose("nvenc:7", &found).map(|e| e.id.as_str()), Some("nvenc:0"));
        assert_eq!(choose("auto", &[]), None, "no GPU: the CPU");
    }

    #[test]
    fn arguments_for_each_kind() {
        let nvenc = nv(1).output_args(1920, 1080);
        assert!(nvenc.windows(2).any(|w| w == ["-gpu", "1"]));
        assert!(nvenc.contains(&"av1_nvenc".to_string()));
        let vaapi = Encoder { id: "x".into(), label: "x".into(), kind: Kind::Vaapi { device: "/dev/dri/renderD129".into(), codec: "av1" } };
        assert_eq!(vaapi.input_args(), ["-vaapi_device", "/dev/dri/renderD129"]);
        let args = vaapi.output_args(1920, 1080);
        let bits: u64 = args[args.iter().position(|a| a == "-b:v").unwrap() + 1].parse().unwrap();
        assert!((2_000_000..3_000_000).contains(&bits), "about 2.5 Mbit/s for 1080p, got {bits}");
    }

    /// Runs the real detection and prints what this computer has. Uses the downloaded ffmpeg,
    /// or the one in `FFMPEG`: `FFMPEG=/path/to/ffmpeg cargo test detect_real -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn detect_real_encoders() {
        let home = std::env::var("HOME").unwrap();
        let ffmpeg = std::env::var("FFMPEG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Path::new(&home).join(".local/share/com.comilito.app/tools/ffmpeg/ffmpeg"));
        let found = detect(&ffmpeg);
        println!("{found:#?}");
    }
}
