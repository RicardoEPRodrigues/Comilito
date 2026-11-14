//! The audio engine: a wrapper around kira that knows how to fade from what's playing now
//! to a new target mix (see mix.rs), plus effects, previews and the transport
//! (play / pause / stop).
//!
//! Signal flow:
//!
//! ```text
//! sound ─┐
//! sound ─┼─> Music track ────┐
//! sound ─┼─> Ambience track ─┼─> main track (master volume) ─> speakers
//! sound ─┴─> Effects track ──┘
//!            previews ───────┘  (straight to main, so they're heard even while paused)
//! ```
//!
//! Volumes are linear multipliers from 0.0 (silent) to 1.0 (full), as in comilito.json.
//! kira wants decibels, so they're converted only when handed to kira.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
    sound::{
        FromFileError, PlaybackState,
        static_sound::{StaticSoundData, StaticSoundHandle},
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
    track::{TrackBuilder, TrackHandle},
};
use serde::Serialize;

use crate::{
    mix::TargetMix,
    project::{Category, Project, Sound},
    storage,
};

/// Volume changes are smoothed over this time. Jumping instantly makes a slider drag
/// sound like small clicks ("zipper noise").
pub const VOLUME_SMOOTHING: Duration = Duration::from_millis(50);

/// kira's handle type for a sound streamed from a file. A type alias is just a shorter name.
type Stream = StreamingSoundHandle<FromFileError>;

/// Play / pause / stop state of the whole scene mix (effects and previews aren't affected).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    /// Nothing from the stack plays. Opening a project starts here; Space starts the mix.
    Stopped,
    Playing,
    /// Music and ambience are frozen in place; Space resumes them.
    Paused,
}

/// What's audible right now, for the UI.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackInfo {
    pub transport: Transport,
    /// The sound id of the music track currently playing, if any.
    pub music_sound: Option<String>,
    /// Sound ids of effects that are currently playing (shown highlighted on the pad).
    pub playing_effects: Vec<String>,
    pub preview: Option<String>,
}

/// Music for one stack entry. Covered music keeps its player, paused, so it resumes where
/// it left off when uncovered.
struct MusicPlayer {
    /// The playlist as the scene had it when this player was made; if the scene's playlist
    /// is edited, the order is rebuilt.
    tracks: Vec<String>,
    /// Play order: indexes into `tracks` (shuffled or not).
    order: Vec<usize>,
    /// Position in `order`.
    position: usize,
    current: Option<(String, Stream)>,
    paused: bool,
}

/// One ambience sound that's part of the mix.
struct AmbiencePlayer {
    handle: Stream,
    volume: f32,
    /// The normalization gain it was last set with, to notice when it changes.
    gain_db: f32,
}

pub struct AudioEngine {
    /// Owns the connection to the sound card. Sounds play only while this is alive.
    manager: AudioManager<DefaultBackend>,
    music_track: TrackHandle,
    ambience_track: TrackHandle,
    effects_track: TrackHandle,

    transport: Transport,

    /// Stack entry id → its music.
    music: HashMap<String, MusicPlayer>,
    /// Sound id → player. A sound plays at most once in the mix (docs/scenes.md).
    ambience: HashMap<String, AmbiencePlayer>,
    effects: Vec<(String, StaticSoundHandle)>,
    preview: Option<(String, Stream)>,
    /// When a range preview should stop by itself (the "loop seam" preview plays across the
    /// loop point for a few seconds, then stops).
    preview_stop_at: Option<Instant>,
    /// Where a range preview starts in the file, so its position can be reported in file time.
    preview_offset: f64,

    /// Effects are loaded into memory once so they start instantly. `StaticSoundData` is
    /// reference counted inside, so cloning it to play again doesn't copy the samples.
    effect_cache: HashMap<PathBuf, StaticSoundData>,
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

        // A closure (an inline function, like a C++ lambda) to avoid writing this three times.
        // It borrows `manager` mutably while it exists.
        let mut add_track = || {
            manager
                .add_sub_track(TrackBuilder::new())
                .map_err(|e| format!("could not create mixer track: {e}"))
        };
        let music_track = add_track()?;
        let ambience_track = add_track()?;
        let effects_track = add_track()?;

        Ok(Self {
            manager,
            music_track,
            ambience_track,
            effects_track,
            transport: Transport::Stopped,
            music: HashMap::new(),
            ambience: HashMap::new(),
            effects: Vec::new(),
            preview: None,
            preview_stop_at: None,
            preview_offset: 0.0,
            effect_cache: HashMap::new(),
        })
    }

    /// Returns the mixer track for a category.
    ///
    /// The return type `&mut TrackHandle` is a *mutable borrow* of one of our fields: the
    /// caller can use it to change the track, but only while `self` is borrowed.
    fn track(&mut self, category: Category) -> &mut TrackHandle {
        match category {
            Category::Music => &mut self.music_track,
            Category::Ambience => &mut self.ambience_track,
            Category::Effects => &mut self.effects_track,
        }
    }

    // ---- Mixer levels -------------------------------------------------------------------

    /// Applies the project's master volume and the mix's category volumes (those of the
    /// top-most enabled scene, decision 0028).
    pub fn apply_levels(&mut self, project: &Project, target: &TargetMix, tween: Duration) {
        self.manager
            .main_track()
            .set_volume(to_decibels(project.master_volume), tween_of(tween));
        for category in Category::ALL {
            let volume = *target.volumes.get(category);
            self.track(category)
                .set_volume(to_decibels(volume), tween_of(tween));
        }
    }

    // ---- The scene mix ------------------------------------------------------------------

    /// Fades from what's playing now to `target`, over `transition`.
    /// Sounds that stay in the mix never restart; they only change volume, pause or resume.
    ///
    /// Returns messages about sounds that couldn't be played (missing files...).
    pub fn apply_mix(
        &mut self,
        project: &Project,
        dir: &Path,
        target: &TargetMix,
        transition: Duration,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        if self.transport == Transport::Stopped {
            return errors;
        }
        self.apply_music(project, dir, target, transition, &mut errors);
        self.apply_ambience(project, dir, target, transition, &mut errors);
        errors
    }

    fn apply_music(
        &mut self,
        project: &Project,
        dir: &Path,
        target: &TargetMix,
        transition: Duration,
        errors: &mut Vec<String>,
    ) {
        // Players for entries that left the stack stop for good. Players for entries still
        // in the stack (but covered or disabled) only pause.
        let in_stack: Vec<&str> = project.stack.iter().map(|e| e.id.as_str()).collect();
        self.music.retain(|entry_id, player| {
            let keep = in_stack.contains(&entry_id.as_str());
            if !keep && let Some((_, handle)) = &mut player.current {
                handle.stop(tween_of(transition));
            }
            keep
        });

        for (entry_id, player) in self.music.iter_mut() {
            if Some(entry_id) != target.music_entry.as_ref() && !player.paused {
                if let Some((_, handle)) = &mut player.current {
                    handle.pause(tween_of(transition));
                }
                player.paused = true;
            }
        }

        let Some(entry_id) = &target.music_entry else {
            return;
        };
        // The target entry is guaranteed to exist and have music; mix.rs only picks such entries.
        let Some(playlist) = project
            .stack
            .iter()
            .find(|e| &e.id == entry_id)
            .and_then(|e| project.scene(&e.scene))
            .and_then(|s| s.music.as_ref())
        else {
            return;
        };
        let shuffle = project.playlist_shuffle(playlist);

        // `entry(...).or_insert_with(...)` gets the existing player or creates one, in one lookup.
        let player = self
            .music
            .entry(entry_id.clone())
            .or_insert_with(|| MusicPlayer::new(&playlist.tracks, shuffle));

        if player.tracks != playlist.tracks {
            // The playlist was edited. Keep the current track if it's still in it.
            let current_id = player.current.as_ref().map(|(id, _)| id.clone());
            // `..MusicPlayer::new(...)` fills every field not listed from a fresh player.
            *player = MusicPlayer {
                current: player.current.take(),
                paused: player.paused,
                ..MusicPlayer::new(&playlist.tracks, shuffle)
            };
            if current_id.is_some_and(|id| !playlist.tracks.contains(&id))
                && let Some((_, mut handle)) = player.current.take()
            {
                handle.stop(tween_of(transition));
            }
        }

        let is_stopped = |p: &MusicPlayer| {
            p.current
                .as_ref()
                .is_none_or(|(_, h)| h.state() == PlaybackState::Stopped)
        };

        if is_stopped(player) {
            // Start (or restart) the current track of the playlist.
            let track_id = player.tracks[player.order[player.position]].clone();
            player.current = None;
            player.paused = false;
            if let Some(sound) = project.sound(&track_id) {
                let looping = playlist.tracks.len() == 1 && project.resolve_sound(sound).looping;
                match start_stream(
                    &mut self.music_track,
                    project,
                    dir,
                    sound,
                    project.resolve_sound(sound).volume,
                    looping,
                    transition,
                ) {
                    Ok(handle) => player.current = Some((track_id, handle)),
                    Err(e) => errors.push(e),
                }
            }
        } else {
            if player.paused {
                if let Some((_, handle)) = &mut player.current {
                    handle.resume(tween_of(transition));
                }
                player.paused = false;
            }
            // Follow edits to the playing track's volume (or a new normalization gain).
            if let Some((id, handle)) = &mut player.current
                && let Some(sound) = project.sound(id)
            {
                let settings = project.resolve_sound(sound);
                handle.set_volume(
                    sound_decibels(settings.volume, settings.gain_db),
                    tween_of(transition.max(VOLUME_SMOOTHING)),
                );
            }
        }
    }

    fn apply_ambience(
        &mut self,
        project: &Project,
        dir: &Path,
        target: &TargetMix,
        transition: Duration,
        errors: &mut Vec<String>,
    ) {
        // Fade out sounds that left the mix.
        self.ambience.retain(|sound_id, player| {
            let keep = target.ambience.contains_key(sound_id);
            if !keep {
                player.handle.stop(tween_of(transition));
            }
            keep
        });

        for (sound_id, &volume) in &target.ambience {
            let gain_db = project
                .sound(sound_id)
                .map_or(0.0, |s| project.resolve_sound(s).gain_db);
            if let Some(player) = self.ambience.get_mut(sound_id) {
                // Already playing (or a one-shot that finished): only change the volume.
                if player.volume != volume || player.gain_db != gain_db {
                    player
                        .handle
                        .set_volume(sound_decibels(volume, gain_db), tween_of(transition));
                    player.volume = volume;
                    player.gain_db = gain_db;
                }
                continue;
            }
            let Some(sound) = project.sound(sound_id) else {
                continue;
            };
            let looping = project.resolve_sound(sound).looping;
            // A layer plays in its sound's category track (normally Ambience).
            let track = match sound.category {
                Category::Music => &mut self.music_track,
                Category::Ambience => &mut self.ambience_track,
                Category::Effects => &mut self.effects_track,
            };
            match start_stream(track, project, dir, sound, volume, looping, transition) {
                Ok(handle) => {
                    self.ambience.insert(
                        sound_id.clone(),
                        AmbiencePlayer {
                            handle,
                            volume,
                            gain_db,
                        },
                    );
                }
                Err(e) => errors.push(e),
            }
        }
    }

    /// Called a few times per second: moves playlists on to their next track when one ends.
    pub fn tick(&mut self, project: &Project, dir: &Path, target: &TargetMix) -> Vec<String> {
        let mut errors = Vec::new();
        self.effects.retain(|(_, h)| h.state() != PlaybackState::Stopped);
        if self.preview_stop_at.is_some_and(|t| Instant::now() >= t) {
            self.stop_preview();
        }
        if self
            .preview
            .as_ref()
            .is_some_and(|(_, h)| h.state() == PlaybackState::Stopped)
        {
            self.preview = None;
        }

        if self.transport != Transport::Playing {
            return errors;
        }
        let Some(entry_id) = &target.music_entry else {
            return errors;
        };
        let Some(player) = self.music.get_mut(entry_id) else {
            return errors;
        };
        let finished = player
            .current
            .as_ref()
            .is_some_and(|(_, h)| h.state() == PlaybackState::Stopped);
        // A single track that doesn't loop plays once and then stays silent.
        if finished && player.tracks.len() > 1 {
            player.advance();
            player.current = None;
            // Re-applying the mix starts the new current track.
            let transition = Duration::ZERO;
            self.apply_music(project, dir, target, transition, &mut errors);
        }
        errors
    }

    // ---- Transport ----------------------------------------------------------------------

    /// Space: start, pause or resume the scene mix.
    pub fn toggle_play(
        &mut self,
        project: &Project,
        dir: &Path,
        target: &TargetMix,
        pause_fade: Duration,
    ) -> Vec<String> {
        match self.transport {
            Transport::Stopped => {
                self.transport = Transport::Playing;
                let transition = Duration::from_millis(project.defaults.transition_ms as u64);
                return self.apply_mix(project, dir, target, transition);
            }
            Transport::Playing => {
                self.transport = Transport::Paused;
                self.music_track.pause(tween_of(pause_fade));
                self.ambience_track.pause(tween_of(pause_fade));
            }
            Transport::Paused => {
                self.transport = Transport::Playing;
                self.music_track.resume(tween_of(pause_fade));
                self.ambience_track.resume(tween_of(pause_fade));
            }
        }
        Vec::new()
    }

    /// Esc: fade everything out and forget positions. The stack itself is kept.
    pub fn stop_all(&mut self, fade: Duration) {
        // While paused, sounds are frozen and silent: a fade would only be heard on resume,
        // so stop them right away and unpause the tracks for the next start.
        let fade = if self.transport == Transport::Paused {
            self.music_track.resume(Tween::default());
            self.ambience_track.resume(Tween::default());
            Duration::ZERO
        } else {
            fade
        };

        // `drain()` empties the map and hands us each value by value (ownership moves out).
        for (_, mut player) in self.music.drain() {
            if let Some((_, mut handle)) = player.current.take() {
                handle.stop(tween_of(fade));
            }
        }
        for (_, mut player) in self.ambience.drain() {
            player.handle.stop(tween_of(fade));
        }
        for (_, mut handle) in self.effects.drain(..) {
            handle.stop(tween_of(fade));
        }
        if let Some((_, mut handle)) = self.preview.take() {
            handle.stop(tween_of(fade));
        }
        self.transport = Transport::Stopped;
    }

    // ---- Effects and previews -----------------------------------------------------------

    /// The effects pad button: plays the effect, or stops it if it's still playing
    /// (with the sound's fade-out). Looping effects therefore work as on/off toggles.
    pub fn toggle_effect(&mut self, project: &Project, dir: &Path, sound: &Sound) -> Result<(), String> {
        let settings = project.resolve_sound(sound);

        let mut stopped_one = false;
        for (id, handle) in &mut self.effects {
            let playing = !matches!(handle.state(), PlaybackState::Stopped | PlaybackState::Stopping);
            if id == &sound.id && playing {
                handle.stop(fade_or_click_guard(settings.fade_out_ms));
                stopped_one = true;
            }
        }
        if stopped_one {
            return Ok(());
        }

        let path = storage::absolute_audio_path(dir, &sound.file);
        // Look in the cache first; load and remember the file if it isn't there.
        let data = match self.effect_cache.get(&path) {
            Some(data) => data.clone(),
            None => {
                let data = StaticSoundData::from_file(&path)
                    .map_err(|e| format!("could not open {}: {e}", sound.name))?;
                self.effect_cache.insert(path, data.clone());
                data
            }
        };

        let mut data = data
            .volume(sound_decibels(settings.volume, settings.gain_db))
            .fade_in_tween(fade_or_none(settings.fade_in_ms));
        if settings.looping {
            data = data.loop_region(..);
        }
        let handle = self
            .track(sound.category)
            .play(data)
            .map_err(|e| format!("could not play {}: {e}", sound.name))?;
        self.effects.push((sound.id.clone(), handle));
        Ok(())
    }

    /// Plays a sound on its own to hear it while building scenes. Calling it again with the
    /// same sound stops it; another sound replaces it.
    pub fn toggle_preview(
        &mut self,
        project: &Project,
        dir: &Path,
        sound: &Sound,
    ) -> Result<(), String> {
        if let Some((id, mut handle)) = self.preview.take() {
            handle.stop(tween_of(Duration::from_millis(200)));
            if id == sound.id {
                return Ok(());
            }
        }
        let path = storage::absolute_audio_path(dir, &sound.file);
        let data = StreamingSoundData::from_file(&path)
            .map_err(|e| format!("could not open {}: {e}", sound.name))?
            .volume({
                let settings = project.resolve_sound(sound);
                sound_decibels(settings.volume, settings.gain_db)
            });
        let handle = self
            .manager
            .play(data)
            .map_err(|e| format!("could not play {}: {e}", sound.name))?;
        self.preview = Some((sound.id.clone(), handle));
        self.preview_offset = 0.0;
        Ok(())
    }

    pub fn stop_preview(&mut self) {
        if let Some((_, mut handle)) = self.preview.take() {
            handle.stop(tween_of(Duration::from_millis(100)));
        }
        self.preview_stop_at = None;
    }

    /// Plays part of a sound on its own, for the crop editor: the selection from `start_ms`
    /// to `end_ms`, or with `seam`, the last two seconds of it followed by the first two (as
    /// a loop would play them), then it stops.
    pub fn preview_range(
        &mut self,
        project: &Project,
        dir: &Path,
        sound: &Sound,
        start_ms: u64,
        end_ms: u64,
        seam: bool,
    ) -> Result<(), String> {
        self.stop_preview();
        let start = start_ms as f64 / 1000.0;
        let end = (end_ms as f64 / 1000.0).max(start + 0.05);
        let path = storage::absolute_audio_path(dir, &sound.file);
        let settings = project.resolve_sound(sound);
        // `slice` makes the sound behave as if the file were only that part.
        let mut data = StreamingSoundData::from_file(&path)
            .map_err(|e| format!("could not open {}: {e}", sound.name))?
            .slice(start..end)
            .volume(sound_decibels(settings.volume, settings.gain_db));
        if seam {
            let length = end - start;
            let lead = 2.0_f64.min(length / 2.0);
            data = data.loop_region(..).start_position(length - lead);
            self.preview_stop_at = Some(Instant::now() + Duration::from_secs_f64(lead * 2.0));
        }
        let handle = self
            .manager
            .play(data)
            .map_err(|e| format!("could not play {}: {e}", sound.name))?;
        self.preview = Some((sound.id.clone(), handle));
        self.preview_offset = start;
        Ok(())
    }

    /// Where the preview is playing, in milliseconds from the start of the file, for the
    /// cursor on the waveform. `None` when nothing is previewed.
    pub fn preview_position(&self) -> Option<(String, u64)> {
        let (id, handle) = self.preview.as_ref()?;
        if handle.state() == PlaybackState::Stopped {
            return None;
        }
        // kira reports positions within the slice, so add where the slice starts.
        Some((id.clone(), ((handle.position() + self.preview_offset) * 1000.0) as u64))
    }

    /// Drops cached effect audio (for example after the project closes or a file changed).
    pub fn clear_cache(&mut self) {
        self.effect_cache.clear();
    }

    /// Whether the scene mix is playing, paused or stopped.
    pub fn playback_transport(&self) -> Transport {
        self.transport
    }

    pub fn playback_info(&self, target: &TargetMix) -> PlaybackInfo {
        let music_sound = target
            .music_entry
            .as_ref()
            .and_then(|entry| self.music.get(entry))
            .and_then(|p| p.current.as_ref())
            .filter(|(_, h)| h.state() != PlaybackState::Stopped)
            .map(|(id, _)| id.clone());
        let mut playing_effects: Vec<String> = self
            .effects
            .iter()
            .filter(|(_, h)| !matches!(h.state(), PlaybackState::Stopped | PlaybackState::Stopping))
            .map(|(id, _)| id.clone())
            .collect();
        playing_effects.sort();
        playing_effects.dedup();
        PlaybackInfo {
            transport: self.transport,
            music_sound,
            playing_effects,
            preview: self.preview.as_ref().map(|(id, _)| id.clone()),
        }
    }
}

impl MusicPlayer {
    fn new(tracks: &[String], shuffle: bool) -> Self {
        let mut order: Vec<usize> = (0..tracks.len()).collect();
        if shuffle {
            shuffle_in_place(&mut order);
        }
        Self {
            tracks: tracks.to_vec(),
            order,
            position: 0,
            current: None,
            paused: false,
        }
    }

    /// Moves to the next track, wrapping around at the end of the playlist.
    fn advance(&mut self) {
        self.position = (self.position + 1) % self.order.len();
    }
}

/// Starts streaming a sound from disk with a fade-in.
fn start_stream(
    track: &mut TrackHandle,
    project: &Project,
    dir: &Path,
    sound: &Sound,
    volume: f32,
    looping: bool,
    fade_in: Duration,
) -> Result<Stream, String> {
    let path = storage::absolute_audio_path(dir, &sound.file);
    let mut data = StreamingSoundData::from_file(&path)
        .map_err(|e| format!("could not open \"{}\": {e}", sound.name))?
        .volume(sound_decibels(volume, project.resolve_sound(sound).gain_db))
        .fade_in_tween(fade_or_none(
            // Use the longer of the scene transition and the sound's own fade-in.
            fade_in.max(Duration::from_millis(project.resolve_sound(sound).fade_in_ms as u64))
                .as_millis() as u32,
        ));
    if looping {
        // `..` is a "full range": loop the whole file.
        data = data.loop_region(..);
    }
    track
        .play(data)
        .map_err(|e| format!("could not play \"{}\": {e}", sound.name))
}

/// Fisher–Yates shuffle. The standard library has no random number generator, so this
/// borrows randomness from `uuid` (which gets it from the OS) instead of adding a crate.
fn shuffle_in_place(items: &mut [usize]) {
    for i in (1..items.len()).rev() {
        let random = uuid::Uuid::new_v4().as_u128();
        let j = (random % (i as u128 + 1)) as usize;
        items.swap(i, j);
    }
}

fn tween_of(duration: Duration) -> Tween {
    Tween {
        duration,
        ..Default::default()
    }
}

/// A fade-in of `ms`, or none at all for 0.
fn fade_or_none(ms: u32) -> Option<Tween> {
    (ms > 0).then(|| tween_of(Duration::from_millis(ms as u64)))
}

/// Stopping instantly can click, so even "no fade" uses kira's tiny default (10 ms).
fn fade_or_click_guard(ms: u32) -> Tween {
    if ms > 0 {
        tween_of(Duration::from_millis(ms as u64))
    } else {
        Tween::default()
    }
}

/// A sound's volume for kira: its linear volume plus its loudness normalization gain.
/// Silence stays silence whatever the gain.
fn sound_decibels(volume: f32, gain_db: f32) -> Decibels {
    let db = to_decibels(volume);
    if db == Decibels::SILENCE {
        db
    } else {
        Decibels(db.0 + gain_db)
    }
}

/// Converts a linear volume (0.0–1.0) to decibels for kira.
///
/// Decibels are logarithmic: halving the amplitude is about −6 dB. kira treats −60 dB and
/// below as silence, so anything that quiet (including 0.0) maps to exactly that.
pub fn to_decibels(volume: f32) -> Decibels {
    if volume <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * volume.min(1.0).log10())
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

    #[test]
    fn shuffle_keeps_every_item_once() {
        let mut items: Vec<usize> = (0..20).collect();
        shuffle_in_place(&mut items);
        let mut sorted = items.clone();
        sorted.sort();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
    }

    #[test]
    fn playlist_wraps_around() {
        let tracks: Vec<String> = vec!["a".into(), "b".into()];
        let mut p = MusicPlayer::new(&tracks, false);
        p.advance();
        assert_eq!(p.position, 1);
        p.advance();
        assert_eq!(p.position, 0);
    }
}
