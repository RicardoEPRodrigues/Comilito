//! Waveform peaks for the sound editor and the crop view (decision 0020).
//!
//! The file is decoded once, as a stream, and reduced to the loudest sample of every 10 ms
//! window. That keeps memory small even for hours-long ambience (a 3-hour file becomes about
//! a million numbers, ~4 MB) instead of the gigabytes the decoded audio would need. The UI
//! then asks for any time range at any width, and `bucket` shrinks the windows to fit.
//!
//! Decoding uses symphonia (the decoder kira itself uses), following the same steps as
//! kira's streaming decoder: probe the format, pick the audio track, decode packet by packet.

use std::{fs::File, path::Path};

use serde::Serialize;
use symphonia::core::{
    audio::{Audio, AudioBuffer, GenericAudioBufferRef, conv::FromSample, sample::Sample},
    codecs::CodecParameters,
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
};

/// Width of one stored window. 10 ms is fine enough to place a cut by eye.
const WINDOW_MS: u64 = 10;

#[derive(Debug, Clone)]
pub struct Peaks {
    /// Loudest absolute sample (0.0–1.0) of each 10 ms window, all channels together.
    pub windows: Vec<f32>,
    pub duration_ms: u64,
}

/// What the UI receives for one view of the waveform.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformView {
    pub duration_ms: u64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub peaks: Vec<f32>,
}

/// Decodes `path` and measures its peaks. Slow for long files; call it off the state lock.
pub fn compute(path: &Path) -> Result<Peaks, String> {
    let fail = |e: &dyn std::fmt::Display| format!("could not read {}: {e}", path.display());
    let file = File::open(path).map_err(|e| fail(&e))?;

    // A hint (the file extension) helps the probe guess the format faster.
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut reader = symphonia::default::get_probe()
        .probe(&hint, stream, Default::default(), Default::default())
        .map_err(|e| fail(&e))?;
    let track = reader
        .default_track(TrackType::Audio)
        .ok_or_else(|| fail(&"no audio track"))?;
    let Some(CodecParameters::Audio(params)) = track.codec_params.as_ref() else {
        return Err(fail(&"no audio track"));
    };
    let sample_rate = params.sample_rate.ok_or_else(|| fail(&"unknown sample rate"))?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &Default::default())
        .map_err(|e| fail(&e))?;
    let track_id = track.id;

    let window_frames = (sample_rate as u64 * WINDOW_MS / 1000).max(1) as usize;
    let mut acc = Accumulator {
        window_frames,
        current_max: 0.0,
        current_frames: 0,
        total_frames: 0,
        windows: Vec::new(),
    };

    // `while let Some(x) = ...` loops until the reader reports the end of the stream.
    while let Some(packet) = reader.next_packet().map_err(|e| fail(&e))? {
        if packet.track_id != track_id {
            continue;
        }
        // A corrupt packet is skipped rather than failing the whole waveform.
        let Ok(buffer) = decoder.decode(&packet) else { continue };
        acc.add(&buffer);
    }
    acc.finish();

    Ok(Peaks {
        duration_ms: acc.total_frames as u64 * 1000 / sample_rate as u64,
        windows: acc.windows,
    })
}

/// Collects the per-window maxima while frames arrive in packets of any size.
struct Accumulator {
    window_frames: usize,
    current_max: f32,
    current_frames: usize,
    total_frames: usize,
    windows: Vec<f32>,
}

impl Accumulator {
    /// symphonia hands back samples in whatever type the file uses (16-bit ints, floats...).
    /// Each variant is converted with the same generic function below.
    fn add(&mut self, buffer: &GenericAudioBufferRef) {
        match buffer {
            GenericAudioBufferRef::U8(b) => self.add_typed(b),
            GenericAudioBufferRef::U16(b) => self.add_typed(b),
            GenericAudioBufferRef::U24(b) => self.add_typed(b),
            GenericAudioBufferRef::U32(b) => self.add_typed(b),
            GenericAudioBufferRef::S8(b) => self.add_typed(b),
            GenericAudioBufferRef::S16(b) => self.add_typed(b),
            GenericAudioBufferRef::S24(b) => self.add_typed(b),
            GenericAudioBufferRef::S32(b) => self.add_typed(b),
            GenericAudioBufferRef::F32(b) => self.add_typed(b),
            GenericAudioBufferRef::F64(b) => self.add_typed(b),
        }
    }

    /// `where f32: FromSample<S>` limits this to sample types that can be converted to f32
    /// (a trait bound, like a C++ concept or a C# generic constraint).
    fn add_typed<S: Sample>(&mut self, buffer: &AudioBuffer<S>)
    where
        f32: FromSample<S>,
    {
        let planes: Vec<&[S]> = (0..buffer.num_planes()).filter_map(|p| buffer.plane(p)).collect();
        let frames = planes.iter().map(|p| p.len()).min().unwrap_or(0);
        for i in 0..frames {
            for plane in &planes {
                let value = f32::from_sample(plane[i]).abs();
                if value > self.current_max {
                    self.current_max = value;
                }
            }
            self.current_frames += 1;
            self.total_frames += 1;
            if self.current_frames == self.window_frames {
                self.windows.push(self.current_max.min(1.0));
                self.current_max = 0.0;
                self.current_frames = 0;
            }
        }
    }

    fn finish(&mut self) {
        if self.current_frames > 0 {
            self.windows.push(self.current_max.min(1.0));
        }
    }
}

impl Peaks {
    /// The peaks between `start_ms` and `end_ms`, shrunk to `buckets` values (each the
    /// loudest window it covers).
    pub fn view(&self, start_ms: u64, end_ms: u64, buckets: usize) -> WaveformView {
        let end_ms = end_ms.min(self.duration_ms).max(start_ms);
        let first = (start_ms / WINDOW_MS) as usize;
        let last = ((end_ms / WINDOW_MS) as usize).min(self.windows.len()).max(first);
        let slice = &self.windows[first.min(self.windows.len())..last];
        WaveformView {
            duration_ms: self.duration_ms,
            start_ms,
            end_ms,
            peaks: bucket(slice, buckets.clamp(1, 4000)),
        }
    }
}

/// Splits `values` into `buckets` equal parts and keeps the maximum of each.
fn bucket(values: &[f32], buckets: usize) -> Vec<f32> {
    if values.is_empty() {
        return vec![0.0; buckets];
    }
    (0..buckets)
        .map(|b| {
            let from = b * values.len() / buckets;
            let to = ((b + 1) * values.len() / buckets).max(from + 1).min(values.len());
            values[from.min(values.len() - 1)..to].iter().copied().fold(0.0, f32::max)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_keep_the_loudest_value() {
        assert_eq!(bucket(&[0.1, 0.9, 0.2, 0.3], 2), vec![0.9, 0.3]);
        // More buckets than values: each value is repeated, nothing panics.
        assert_eq!(bucket(&[0.5, 0.7], 4).len(), 4);
        assert_eq!(bucket(&[], 3), vec![0.0; 3]);
    }

    #[test]
    fn decodes_the_bundled_sample() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/sample/effects/tolling-bell.mp3");
        let peaks = compute(&path).expect("sample decodes");
        assert!(peaks.duration_ms > 1000, "{} ms", peaks.duration_ms);
        assert!(peaks.windows.iter().any(|&p| p > 0.1), "has audible content");
        let view = peaks.view(0, peaks.duration_ms, 100);
        assert_eq!(view.peaks.len(), 100);
    }
}

#[cfg(test)]
mod m4a_check {
    /// `COMILITO_M4A=/path/file.m4a cargo test decodes_m4a -- --ignored`
    #[test]
    #[ignore]
    fn decodes_m4a() {
        let path = std::path::PathBuf::from(std::env::var("COMILITO_M4A").expect("set COMILITO_M4A"));
        let peaks = super::compute(&path).expect("symphonia decodes AAC in MP4");
        assert!((5500..=6500).contains(&peaks.duration_ms), "{} ms", peaks.duration_ms);
        // kira (the audio engine) must be able to open it too.
        kira::sound::streaming::StreamingSoundData::from_file(&path).expect("kira opens m4a");
        crate::loudness::measure(&path).expect("loudness of m4a");
    }
}
