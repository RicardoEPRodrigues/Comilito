//! The audio engine: a thin wrapper around kira.
//!
//! Signal flow:
//!
//! ```text
//! sound ─┐
//! sound ─┼─> Music track ────┐
//! sound ─┼─> Ambience track ─┼─> main track (master volume) ─> speakers
//! sound ─┴─> Effects track ──┘
//! ```
//!
//! Volumes are linear multipliers from 0.0 (silent) to 1.0 (full), as in comilito.json.
//! kira wants decibels, so they're converted only when handed to kira.

use std::time::Duration;

use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
    sound::{
        FromFileError, PlaybackState,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
    track::{TrackBuilder, TrackHandle},
};
use serde::Serialize;

use crate::project::Category;

/// Fade used by the temporary `play_file` command, so starts and stops don't click.
const FADE: Duration = Duration::from_millis(500);

/// Volume changes are smoothed over this time. Jumping instantly makes a slider drag
/// sound like small clicks ("zipper noise").
const VOLUME_SMOOTHING: Duration = Duration::from_millis(50);

/// Current mixer volumes, sent to the UI so its sliders start in the right place.
/// kira's handles can set a volume but not read it back, so the engine keeps its own copy.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub ambience: f32,
    pub effects: f32,
}

impl Default for Volumes {
    /// The app's default levels from docs/preferences.md. These will come from the
    /// project once there is one.
    fn default() -> Self {
        Self {
            master: 0.9,
            music: 0.8,
            ambience: 0.6,
            effects: 1.0,
        }
    }
}

pub struct AudioEngine {
    /// Owns the connection to the sound card. Sounds play only while this is alive.
    manager: AudioManager<DefaultBackend>,

    music: TrackHandle,
    ambience: TrackHandle,
    effects: TrackHandle,

    volumes: Volumes,

    /// Handles to the sounds we've started, so we can stop them later.
    /// Dropping a handle does *not* stop its sound in kira; the handle is just a remote control.
    playing: Vec<StreamingSoundHandle<FromFileError>>,
}

impl AudioEngine {
    /// Opens the default audio output device and builds the category tracks.
    ///
    /// Returns `Result` because this can fail (no sound card, device busy...).
    /// `Result<T, E>` is Rust's replacement for exceptions: the caller *must* handle
    /// the error case before it can use the value.
    pub fn new() -> Result<Self, String> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| format!("could not open audio output: {e}"))?;

        let volumes = Volumes::default();

        // A closure (an inline function, like a C++ lambda) to avoid writing this three times.
        // `|volume: f32|` is its parameter list. It borrows `manager` mutably while it exists.
        let mut add_track = |volume: f32| {
            manager
                .add_sub_track(TrackBuilder::new().volume(to_decibels(volume)))
                .map_err(|e| format!("could not create mixer track: {e}"))
        };
        let music = add_track(volumes.music)?;
        let ambience = add_track(volumes.ambience)?;
        let effects = add_track(volumes.effects)?;

        manager
            .main_track()
            .set_volume(to_decibels(volumes.master), Tween::default());

        Ok(Self {
            manager,
            music,
            ambience,
            effects,
            volumes,
            playing: Vec::new(),
        })
    }

    /// Returns the mixer track for a category.
    ///
    /// The return type `&mut TrackHandle` is a *mutable borrow* of one of our fields: the
    /// caller can use it to change the track, but only while `self` is borrowed, so the
    /// engine can't be moved or used elsewhere in the meantime.
    fn track(&mut self, category: Category) -> &mut TrackHandle {
        match category {
            Category::Music => &mut self.music,
            Category::Ambience => &mut self.ambience,
            Category::Effects => &mut self.effects,
        }
    }

    pub fn volumes(&self) -> Volumes {
        self.volumes
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        self.volumes.master = volume;
        self.manager.main_track().set_volume(
            to_decibels(volume),
            Tween {
                duration: VOLUME_SMOOTHING,
                ..Default::default()
            },
        );
    }

    pub fn set_category_volume(&mut self, category: Category, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        match category {
            Category::Music => self.volumes.music = volume,
            Category::Ambience => self.volumes.ambience = volume,
            Category::Effects => self.volumes.effects = volume,
        }
        self.track(category).set_volume(
            to_decibels(volume),
            Tween {
                duration: VOLUME_SMOOTHING,
                ..Default::default()
            },
        );
    }

    /// Streams an audio file from disk into a category's track, fading it in.
    ///
    /// `&mut self` means this method borrows the engine mutably: nobody else can touch it
    /// until the method returns. The compiler enforces that, which is why commands lock a
    /// `Mutex` before calling this (see lib.rs).
    pub fn play_file(&mut self, path: &str, category: Category) -> Result<(), String> {
        // `?` means: if this is an error, return it from play_file right now;
        // otherwise unwrap the value. Like Go's `if err != nil { return err }` in one character.
        // `map_err` first turns kira's error type into a readable String.
        let sound = StreamingSoundData::from_file(path)
            .map_err(|e| format!("could not open {path}: {e}"))?
            .fade_in_tween(Tween {
                duration: FADE,
                ..Default::default()
            });

        let handle = self
            .track(category)
            .play(sound)
            .map_err(|e| format!("could not play {path}: {e}"))?;

        // Forget handles of sounds that already finished, so the list doesn't grow forever.
        self.playing.retain(|h| h.state() != PlaybackState::Stopped);
        self.playing.push(handle);
        Ok(())
    }

    /// Fades out and stops every sound. The "panic button".
    pub fn stop_all(&mut self) {
        // `drain(..)` empties the Vec and hands us each handle by value (ownership moves to `h`).
        for mut h in self.playing.drain(..) {
            h.stop(Tween {
                duration: FADE,
                ..Default::default()
            });
        }
    }
}

/// Converts a linear volume (0.0–1.0) to decibels for kira.
///
/// Decibels are logarithmic: halving the amplitude is about −6 dB. kira treats −60 dB and
/// below as silence, so anything that quiet (including 0.0) maps to exactly that.
fn to_decibels(volume: f32) -> Decibels {
    if volume <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * volume.log10())
    }
}

// Unit tests live next to the code they test. `#[cfg(test)]` means this module is only
// compiled for `cargo test`, never into the app. Run with:
//   cargo test --manifest-path src-tauri/Cargo.toml
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_volume_is_zero_decibels() {
        assert_eq!(to_decibels(1.0), Decibels::IDENTITY);
    }

    #[test]
    fn half_volume_is_about_minus_six_decibels() {
        let db = to_decibels(0.5).0;
        assert!((db - -6.02).abs() < 0.01, "got {db}");
    }

    #[test]
    fn zero_and_tiny_volumes_are_silence() {
        assert_eq!(to_decibels(0.0), Decibels::SILENCE);
        assert_eq!(to_decibels(0.0005), Decibels::SILENCE);
    }

    #[test]
    fn round_trips_through_kira() {
        for v in [0.1, 0.3, 0.6, 0.9] {
            let back = to_decibels(v).as_amplitude();
            assert!((back - v).abs() < 0.0001, "{v} came back as {back}");
        }
    }
}
