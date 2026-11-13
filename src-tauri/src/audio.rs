//! The audio engine: a thin wrapper around kira.
//!
//! For now it can only play a file by path and stop everything. Category tracks,
//! volumes and per-sound settings come once the project model exists.

use std::time::Duration;

use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, Tween,
    sound::{
        FromFileError,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
};

/// Fade used by the temporary `play_file` command, so starts and stops don't click.
const FADE: Duration = Duration::from_millis(500);

pub struct AudioEngine {
    /// Owns the connection to the sound card. Sounds play only while this is alive.
    manager: AudioManager<DefaultBackend>,

    /// Handles to the sounds we've started, so we can stop them later.
    /// Dropping a handle does *not* stop its sound in kira; the handle is just a remote control.
    playing: Vec<StreamingSoundHandle<FromFileError>>,
}

impl AudioEngine {
    /// Opens the default audio output device.
    ///
    /// Returns `Result` because this can fail (no sound card, device busy...).
    /// `Result<T, E>` is Rust's replacement for exceptions: the caller *must* handle
    /// the error case before it can use the value.
    pub fn new() -> Result<Self, String> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|e| format!("could not open audio output: {e}"))?;

        Ok(Self {
            manager,
            playing: Vec::new(),
        })
    }

    /// Streams an audio file from disk, fading it in.
    ///
    /// `&mut self` means this method borrows the engine mutably: nobody else can touch it
    /// until the method returns. The compiler enforces that, which is why commands lock a
    /// `Mutex` before calling this (see lib.rs).
    pub fn play_file(&mut self, path: &str) -> Result<(), String> {
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
            .manager
            .play(sound)
            .map_err(|e| format!("could not play {path}: {e}"))?;

        // Forget handles of sounds that already finished, so the list doesn't grow forever.
        self.playing
            .retain(|h| h.state() != kira::sound::PlaybackState::Stopped);
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
