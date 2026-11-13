//! Measuring how loud an audio file sounds, for loudness normalization.
//!
//! Peak level says little about how loud something *sounds*: a quiet recording with one
//! loud click has a high peak. EBU R128 loudness (in LUFS) weights frequencies the way
//! ears do and averages over the whole file, ignoring silence. It's what broadcasters and
//! streaming services use to make one programme as loud as the next. The `ebur128` crate
//! implements the standard; we only feed it the decoded samples.

use std::path::Path;

use ebur128::{EbuR128, Mode};
use kira::sound::static_sound::StaticSoundData;

use crate::project::Loudness;

/// Decodes `path` and measures its integrated loudness and sample peak.
///
/// Runs on the background ticker thread (it can take a second or two for a long track).
/// It decodes the whole file into memory first (kira's decoder works that way); that's
/// briefly ~100 MB for a 4-minute track, freed as soon as the measurement is done.
pub fn measure(path: &Path) -> Result<Loudness, String> {
    let data = StaticSoundData::from_file(path)
        .map_err(|e| format!("could not decode {}: {e}", path.display()))?;

    // kira gives stereo frames even for mono files (both channels the same), which is also
    // how the file is heard, so we always measure two channels.
    let mut meter = EbuR128::new(2, data.sample_rate, Mode::I | Mode::SAMPLE_PEAK)
        .map_err(|e| format!("could not measure loudness: {e:?}"))?;

    // The meter wants interleaved samples (L R L R ...). Convert a chunk at a time, so we
    // don't make a second full-size copy of the file.
    let mut interleaved = Vec::with_capacity(2 * 4096);
    for chunk in data.frames.chunks(4096) {
        interleaved.clear();
        for frame in chunk {
            interleaved.push(frame.left);
            interleaved.push(frame.right);
        }
        meter
            .add_frames_f32(&interleaved)
            .map_err(|e| format!("could not measure loudness: {e:?}"))?;
    }

    let lufs = meter
        .loudness_global()
        .map_err(|e| format!("could not measure loudness: {e:?}"))?;
    let peak = (0..2)
        .filter_map(|ch| meter.sample_peak(ch).ok())
        .fold(0.0_f64, f64::max);

    Ok(Loudness {
        // A silent file measures as −∞; store a floor instead (JSON has no infinity).
        lufs: if lufs.is_finite() { lufs as f32 } else { -70.0 },
        peak: peak as f32,
        duration_ms: Some(data.duration().as_millis() as u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    /// Writes a 3-second 1 kHz mono sine wave as a 16-bit WAV file.
    fn write_sine(path: &Path, amplitude: f32) {
        let rate = 44_100u32;
        let samples: Vec<i16> = (0..rate * 3)
            .map(|i| {
                let t = i as f32 / rate as f32;
                (amplitude * (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * i16::MAX as f32) as i16
            })
            .collect();
        let data_len = (samples.len() * 2) as u32;
        // A minimal WAV header: "RIFF", format chunk (PCM, 1 channel, 16 bit), data chunk.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes()); // bytes per second
        bytes.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn half_the_amplitude_measures_six_lu_quieter() {
        let dir = TempDir::new();
        let loud = dir.0.join("loud.wav");
        let quiet = dir.0.join("quiet.wav");
        write_sine(&loud, 0.5);
        write_sine(&quiet, 0.25);

        let loud = measure(&loud).unwrap();
        let quiet = measure(&quiet).unwrap();

        let difference = loud.lufs - quiet.lufs;
        assert!((difference - 6.02).abs() < 0.1, "difference was {difference}");
        assert!((loud.peak - 0.5).abs() < 0.01, "peak was {}", loud.peak);
    }

    #[test]
    fn unreadable_file_is_an_error() {
        let dir = TempDir::new();
        let path = dir.0.join("not-audio.mp3");
        std::fs::write(&path, b"hello").unwrap();
        assert!(measure(&path).is_err());
    }
}
