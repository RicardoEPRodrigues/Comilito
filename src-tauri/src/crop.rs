//! Cutting audio files with ffmpeg (decision 0020). Used by the crop editor and to cut a time
//! range out of a direct-link download.
//!
//! By default the audio is copied without re-encoding (`-c copy`): no quality is lost, and
//! the cut lands on the nearest audio frame, a few milliseconds from the mark. Fading the
//! edges or asking for an exact cut re-encodes the file in its own format at high quality.

use std::path::Path;

use serde::Deserialize;

use crate::tools::hidden_command;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CutOptions {
    pub start_ms: u64,
    pub end_ms: u64,
    /// A 10 ms fade at both ends, so the cut doesn't click.
    pub fade_edges: bool,
    /// Re-encode for a sample-exact cut.
    pub exact: bool,
}

/// ffmpeg's encoder settings for re-encoding a file in its own format, by extension.
fn encoder_args(ext: &str) -> Result<&'static [&'static str], String> {
    Ok(match ext {
        "mp3" => &["-c:a", "libmp3lame", "-q:a", "2"],
        "ogg" | "oga" => &["-c:a", "libvorbis", "-q:a", "6"],
        "flac" => &["-c:a", "flac"],
        "wav" => &["-c:a", "pcm_s16le"],
        "m4a" | "aac" => &["-c:a", "aac", "-b:a", "192k"],
        other => return Err(format!("can't re-encode .{other} files")),
    })
}

pub(crate) fn seconds(ms: u64) -> String {
    format!("{:.3}", ms as f64 / 1000.0)
}

/// Writes the part of `input` between `start_ms` and `end_ms` to `output` (same format).
pub fn cut(ffmpeg: &Path, input: &Path, output: &Path, options: CutOptions) -> Result<(), String> {
    if options.end_ms <= options.start_ms {
        return Err("the end must be after the start".into());
    }
    let ext = output
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let duration_ms = options.end_ms - options.start_ms;

    let mut cmd = hidden_command(ffmpeg);
    cmd.args(["-hide_banner", "-loglevel", "error", "-y"])
        // `-ss` before `-i` seeks in the input quickly; `-t` is the length to keep.
        .args(["-ss", &seconds(options.start_ms)])
        .arg("-i")
        .arg(input)
        .args(["-t", &seconds(duration_ms)])
        // Audio only (drops cover art and video), keeping the tags.
        .args(["-vn", "-map_metadata", "0"]);

    if options.fade_edges || options.exact {
        cmd.args(encoder_args(&ext)?);
        if options.fade_edges {
            let fade = 0.01_f64.min(duration_ms as f64 / 2000.0);
            let out_start = duration_ms as f64 / 1000.0 - fade;
            cmd.args([
                "-af",
                &format!("afade=t=in:st=0:d={fade:.3},afade=t=out:st={out_start:.3}:d={fade:.3}"),
            ]);
        }
    } else {
        cmd.args(["-c", "copy"]);
    }
    if ext == "m4a" {
        cmd.args(["-movflags", "+faststart"]);
    }
    cmd.arg(output);

    let out = cmd.output().map_err(|e| format!("could not run ffmpeg: {e}"))?;
    if !out.status.success() {
        let message = String::from_utf8_lossy(&out.stderr);
        return Err(format!("ffmpeg could not cut the file: {}", message.trim()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_format_has_an_encoder() {
        for ext in ["mp3", "ogg", "flac", "wav", "m4a"] {
            assert!(encoder_args(ext).is_ok(), "{ext}");
        }
        assert!(encoder_args("xyz").is_err());
    }

    /// Needs the tools installed by `tools::tests::install_real_tools` into the app data
    /// folder, or `COMILITO_FFMPEG` pointing at an ffmpeg. Run with `--ignored`.
    #[test]
    #[ignore]
    fn cuts_the_sample_with_real_ffmpeg() {
        let ffmpeg = std::env::var("COMILITO_FFMPEG").expect("set COMILITO_FFMPEG");
        let dir = crate::storage::test_support::TempDir::new();
        let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/sample/effects/tolling-bell.mp3");
        for (name, fade, exact) in [("copy.mp3", false, false), ("fade.mp3", true, false), ("exact.mp3", false, true)] {
            let output = dir.0.join(name);
            cut(
                Path::new(&ffmpeg),
                &input,
                &output,
                CutOptions { start_ms: 500, end_ms: 2500, fade_edges: fade, exact },
            )
            .unwrap();
            let peaks = crate::waveform::compute(&output).unwrap();
            assert!((1800..=2200).contains(&peaks.duration_ms), "{name}: {} ms", peaks.duration_ms);
        }
    }
}
