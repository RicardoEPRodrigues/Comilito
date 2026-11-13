//! The project data model: Rust structs that mirror `comilito.json` (docs/save-format.md).
//!
//! serde turns these structs into JSON and back. A few attributes appear throughout:
//! - `#[serde(rename_all = "camelCase")]`: Rust uses `snake_case` field names, JSON uses
//!   `camelCase`, so `fade_in_ms` is written as `fadeInMs`.
//! - `#[serde(default)]`: if the field is missing from the file, use its default
//!   (empty list, `None`...) instead of failing. This keeps older or hand-edited files loadable.
//! - `#[serde(skip_serializing_if = "Option::is_none")]`: don't write the field when it's
//!   `None`. Optional per-sound/per-scene settings are only stored when they override the
//!   project default (docs/preferences.md).
//!
//! `Option<T>` is Rust's "maybe a value": either `Some(value)` or `None`. There is no null in
//! safe Rust, so anything that can be absent says so in its type, and the compiler makes you
//! handle the `None` case.

use serde::{Deserialize, Serialize};

/// The save format version this build writes. See `storage::load_project` for migrations.
pub const FORMAT_VERSION: u32 = 2;

/// The three kinds of sound. Each has its own mixer track and volume.
///
/// A Rust `enum` like this one is a closed set of values, like a C# enum. `match` on it must
/// handle every variant, so adding a fourth category later makes the compiler point at
/// every place that needs updating.
///
/// The `derive` line asks the compiler to generate common behaviour:
/// - `Clone, Copy`: copying a Category is as cheap as copying an int, so it's passed by value.
/// - `PartialEq, Eq, Hash`: allows `==` and using it as a HashMap key.
/// - `Debug`: printable with `{:?}` for logging.
/// - `Serialize, Deserialize` (from serde): converts to and from JSON.
///   `rename_all = "lowercase"` makes `Category::Music` appear as `"music"` in JSON, which is
///   what the frontend sends and what comilito.json uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Music,
    Ambience,
    Effects,
}

impl Category {
    pub const ALL: [Category; 3] = [Category::Music, Category::Ambience, Category::Effects];

    /// Folder name under `audio/`, and the name used in JSON.
    pub fn folder_name(self) -> &'static str {
        // `&'static str` is a string slice that lives for the whole program (a literal baked
        // into the binary), so the caller can keep it as long as it likes.
        match self {
            Category::Music => "music",
            Category::Ambience => "ambience",
            Category::Effects => "effects",
        }
    }
}

/// The whole contents of `comilito.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub format_version: u32,
    pub name: String,
    pub master_volume: f32,
    pub categories: PerCategory<CategoryMix>,
    #[serde(default)]
    pub sounds: Vec<Sound>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
    /// Ordered bottom to top.
    #[serde(default)]
    pub stack: Vec<StackEntry>,
    #[serde(default)]
    pub stack_presets: Vec<StackPreset>,
    /// Preset groups in display order. A group also exists while a preset names it; this
    /// list adds the order and keeps groups that are still empty ("New group").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preset_groups: Vec<String>,
    /// The preset the current stack was loaded from (or last saved to), to show
    /// "From Forest Ambush · modified" and offer Update / Revert (decision 0017).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_origin: Option<String>,
    pub defaults: ProjectDefaults,
    #[serde(default)]
    pub backups: BackupSettings,
}

/// One value per category. Generic over `T` (like a C# generic or C++ template) so the same
/// shape serves the category volumes and the per-category sound defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerCategory<T> {
    pub music: T,
    pub ambience: T,
    pub effects: T,
}

impl<T> PerCategory<T> {
    pub fn get(&self, category: Category) -> &T {
        match category {
            Category::Music => &self.music,
            Category::Ambience => &self.ambience,
            Category::Effects => &self.effects,
        }
    }

    pub fn get_mut(&mut self, category: Category) -> &mut T {
        match category {
            Category::Music => &mut self.music,
            Category::Ambience => &mut self.ambience,
            Category::Effects => &mut self.effects,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMix {
    pub volume: f32,
}

/// An audio file in the library, plus its own settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: String,
    pub category: Category,
    pub name: String,
    pub file: FileInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
    // `loop` is a Rust keyword, so the field is called `looping` in Rust and `loop` in JSON.
    #[serde(rename = "loop", default, skip_serializing_if = "Option::is_none")]
    pub looping: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out_ms: Option<u32>,
    /// Free-form labels for searching ("forest", "combat"), without the `#`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// `skip_serializing_if = "std::ops::Not::not"` means "don't write it when false".
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub favourite: bool,
    /// Where a downloaded sound came from (decision 0019).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// When the sound was last previewed, played on the pad or added to a scene (RFC 3339),
    /// for the Library's "Recent" list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<String>,
}

/// Provenance of a downloaded sound, so it can be opened or fetched again.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The site, e.g. "youtube" or the host name of a direct link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    pub downloaded_at: String,
    /// The part of the original that was downloaded, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<TimeRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    /// Relative to the project folder, always with `/` separators (also on Windows),
    /// so the file reads the same on every OS.
    pub path: String,
    pub size: u64,
    /// RFC 3339 timestamp of the file's modification time.
    pub modified: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Measured in the background (see loudness.rs). `None` until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loudness: Option<Loudness>,
}

/// How loud a file is, measured once with EBU R128 (the broadcast loudness standard).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loudness {
    /// Integrated loudness in LUFS (around −14 for pop music, −30 for quiet ambience).
    pub lufs: f32,
    /// Highest sample, linear (1.0 = the loudest a file can be without clipping).
    pub peak: f32,
}

/// Loudness normalization: which categories are evened out, and to what level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Normalization {
    pub music: bool,
    pub ambience: bool,
    pub effects: bool,
    pub target_lufs: f32,
}

impl Default for Normalization {
    fn default() -> Self {
        Self {
            music: true,
            ambience: true,
            effects: false,
            target_lufs: -18.0,
        }
    }
}

impl Normalization {
    pub fn applies_to(&self, category: Category) -> bool {
        match category {
            Category::Music => self.music,
            Category::Ambience => self.ambience,
            Category::Effects => self.effects,
        }
    }
}

/// The most normalization may turn a quiet file up. Turning loud files down has no limit.
pub const MAX_NORMALIZATION_BOOST_DB: f32 = 12.0;

/// A saved mix: what plays together. See docs/scenes.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    /// `None` (JSON `null`) when the scene has no music, which leaves lower music playing
    /// (unless `blocks_music` is set).
    #[serde(default)]
    pub music: Option<Playlist>,
    /// For a scene without music: silence the music of the scenes below it while it's
    /// enabled, instead of letting it play through. Only written when true.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub blocks_music: bool,
    #[serde(default)]
    pub ambience: Vec<Layer>,
    /// Sound ids shown on the effects pad.
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duck_level: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    /// Sound ids, played in order (or shuffled).
    pub tracks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shuffle: Option<bool>,
}

/// One ambience sound in a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub sound: String,
    /// Volume in this scene. If missing, the sound's own volume is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
}

/// Lets an ambience layer escape ducking; see "Ambience and ducking" in docs/scenes.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Foreground,
    Background,
    Supporting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackEntry {
    /// Each entry has its own id, so the same scene can be in the stack twice.
    pub id: String,
    pub scene: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPreset {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    pub stack: Vec<PresetEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetEntry {
    pub scene: String,
    pub enabled: bool,
}

/// Project-wide defaults that scenes and sounds inherit (docs/preferences.md).
/// Always written in full, so the project sounds the same on every computer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDefaults {
    pub duck_level: f32,
    pub transition_ms: u32,
    pub sound_volume: f32,
    pub categories: PerCategory<SoundDefaults>,
    /// Added after the first projects were made, so it needs a default when missing.
    #[serde(default)]
    pub normalize: Normalization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundDefaults {
    #[serde(rename = "loop")]
    pub looping: bool,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    /// Only meaningful for music.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shuffle: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub interval_minutes: u32,
    pub keep: u32,
    pub include_audio: bool,
}

// `impl Default` is how a type says what its "empty" value is. `#[serde(default)]` on a
// field of this type uses it when the field is missing.
impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            interval_minutes: 30,
            keep: 1,
            include_audio: false,
        }
    }
}

impl Default for ProjectDefaults {
    /// The starting values from docs/preferences.md.
    fn default() -> Self {
        Self {
            duck_level: 0.3,
            transition_ms: 2500,
            sound_volume: 1.0,
            categories: PerCategory {
                music: SoundDefaults {
                    looping: true,
                    fade_in_ms: 2000,
                    fade_out_ms: 2000,
                    shuffle: Some(false),
                },
                ambience: SoundDefaults {
                    looping: true,
                    fade_in_ms: 1500,
                    fade_out_ms: 1500,
                    shuffle: None,
                },
                effects: SoundDefaults {
                    looping: false,
                    fade_in_ms: 0,
                    fade_out_ms: 0,
                    shuffle: None,
                },
            },
            normalize: Normalization::default(),
        }
    }
}

/// Sound settings with every inherited value filled in. The audio engine only ever sees these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedSound {
    pub volume: f32,
    pub looping: bool,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    /// Loudness normalization gain in decibels, added on top of `volume` (0 when off).
    pub gain_db: f32,
}

impl Project {
    /// A new, empty project using the given defaults.
    pub fn new(name: &str, defaults: ProjectDefaults) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            name: name.to_string(),
            master_volume: 1.0,
            categories: PerCategory {
                music: CategoryMix { volume: 0.8 },
                ambience: CategoryMix { volume: 0.6 },
                effects: CategoryMix { volume: 1.0 },
            },
            sounds: Vec::new(),
            scenes: Vec::new(),
            stack: Vec::new(),
            stack_presets: Vec::new(),
            preset_groups: Vec::new(),
            stack_origin: None,
            defaults,
            backups: BackupSettings::default(),
        }
    }

    /// Finds a sound by id. Returns `Option<&Sound>`: a borrowed reference into the
    /// project's list (no copy), or `None` if there's no such sound.
    pub fn sound(&self, id: &str) -> Option<&Sound> {
        self.sounds.iter().find(|s| s.id == id)
    }

    pub fn sound_mut(&mut self, id: &str) -> Option<&mut Sound> {
        self.sounds.iter_mut().find(|s| s.id == id)
    }

    pub fn scene(&self, id: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.id == id)
    }

    pub fn preset(&self, id: &str) -> Option<&StackPreset> {
        self.stack_presets.iter().find(|p| p.id == id)
    }

    /// The stack as preset entries (stack entries without their ids).
    pub fn stack_as_preset_entries(&self) -> Vec<PresetEntry> {
        self.stack
            .iter()
            .map(|e| PresetEntry {
                scene: e.scene.clone(),
                enabled: e.enabled,
            })
            .collect()
    }

    /// Where the current stack stands compared with the presets (decision 0017).
    pub fn stack_status(&self) -> StackStatus {
        if self.stack.is_empty() {
            return StackStatus::Empty;
        }
        let current = self.stack_as_preset_entries();
        if let Some(origin) = self.stack_origin.as_deref().and_then(|id| self.preset(id)) {
            return if origin.stack == current {
                StackStatus::Saved
            } else {
                StackStatus::Modified
            };
        }
        // Not loaded from a preset, but it may happen to match one exactly.
        if self.stack_presets.iter().any(|p| p.stack == current) {
            StackStatus::Saved
        } else {
            StackStatus::Unsaved
        }
    }

    /// Every preset group, in display order: the stored order first, then any group a
    /// preset names that isn't in the list yet (for example, from a hand-edited file).
    pub fn all_preset_groups(&self) -> Vec<String> {
        let mut groups = self.preset_groups.clone();
        for preset in &self.stack_presets {
            if let Some(g) = &preset.group
                && !groups.contains(g)
            {
                groups.push(g.clone());
            }
        }
        groups
    }

    /// A sound's settings with project defaults filled in for anything it doesn't set.
    pub fn resolve_sound(&self, sound: &Sound) -> ResolvedSound {
        let d = self.defaults.categories.get(sound.category);
        // `unwrap_or(x)` takes the value out of `Some`, or uses `x` for `None`.
        ResolvedSound {
            volume: sound.volume.unwrap_or(self.defaults.sound_volume),
            looping: sound.looping.unwrap_or(d.looping),
            fade_in_ms: sound.fade_in_ms.unwrap_or(d.fade_in_ms),
            fade_out_ms: sound.fade_out_ms.unwrap_or(d.fade_out_ms),
            gain_db: self.normalization_gain_db(sound),
        }
    }

    /// How many decibels to turn a sound up (positive) or down (negative) so it plays at the
    /// project's target loudness. 0 if normalization is off for its category or it hasn't
    /// been measured yet.
    pub fn normalization_gain_db(&self, sound: &Sound) -> f32 {
        let n = &self.defaults.normalize;
        let Some(loudness) = sound.file.loudness else {
            return 0.0;
        };
        // Silent or near-silent files can't be meaningfully normalized.
        if !n.applies_to(sound.category) || !loudness.lufs.is_finite() || loudness.lufs < -60.0 {
            return 0.0;
        }
        let wanted = n.target_lufs - loudness.lufs;
        // Never boost a file past the point where its loudest sample would clip.
        let headroom = if loudness.peak > 0.0 {
            -20.0 * loudness.peak.log10()
        } else {
            0.0
        };
        if wanted > 0.0 {
            wanted.min(headroom.max(0.0)).min(MAX_NORMALIZATION_BOOST_DB)
        } else {
            wanted
        }
    }

    pub fn scene_duck_level(&self, scene: &Scene) -> f32 {
        scene.duck_level.unwrap_or(self.defaults.duck_level)
    }

    pub fn scene_transition_ms(&self, scene: &Scene) -> u32 {
        scene.transition_ms.unwrap_or(self.defaults.transition_ms)
    }

    pub fn playlist_shuffle(&self, playlist: &Playlist) -> bool {
        playlist
            .shuffle
            .or(self.defaults.categories.music.shuffle)
            .unwrap_or(false)
    }

    /// Takes a sound out of one scene (its playlist, layers and effects pad).
    pub fn remove_sound_from_scene(&mut self, scene_id: &str, sound_id: &str) {
        if let Some(scene) = self.scenes.iter_mut().find(|s| s.id == scene_id) {
            if let Some(music) = &mut scene.music {
                music.tracks.retain(|t| t != sound_id);
                // An emptied playlist means "no music" (lower music plays again).
                if music.tracks.is_empty() {
                    scene.music = None;
                }
            }
            scene.ambience.retain(|l| l.sound != sound_id);
            scene.effects.retain(|e| e != sound_id);
        }
    }

    /// Takes a sound out of the scene slots that don't hold `category`: music playlists only
    /// hold music, ambience layers only ambience, effects pads only effects.
    pub fn remove_sound_from_other_slots(&mut self, id: &str, category: Category) {
        for scene in &mut self.scenes {
            if category != Category::Music
                && let Some(music) = &mut scene.music
            {
                music.tracks.retain(|t| t != id);
            }
            if category != Category::Ambience {
                scene.ambience.retain(|l| l.sound != id);
            }
            if category != Category::Effects {
                scene.effects.retain(|e| e != id);
            }
        }
    }

    /// Removes every reference to sounds or scenes that don't exist any more, so the rest
    /// of the code can assume ids always point somewhere. Called after loading (the file
    /// may have been edited by hand) and after deleting things.
    pub fn remove_dangling_references(&mut self) {
        // Collect the valid ids first. We can't look them up in `self.sounds` while also
        // mutating `self.scenes` inside the loop below: the borrow checker forbids holding
        // a shared borrow of `self` and a mutable one at the same time.
        let sound_ids: std::collections::HashSet<String> =
            self.sounds.iter().map(|s| s.id.clone()).collect();
        let scene_ids: std::collections::HashSet<String> =
            self.scenes.iter().map(|s| s.id.clone()).collect();

        for scene in &mut self.scenes {
            if let Some(music) = &mut scene.music {
                music.tracks.retain(|id| sound_ids.contains(id));
            }
            scene.ambience.retain(|l| sound_ids.contains(&l.sound));
            scene.effects.retain(|id| sound_ids.contains(id));
        }
        self.stack.retain(|e| scene_ids.contains(&e.scene));
        for preset in &mut self.stack_presets {
            preset.stack.retain(|e| scene_ids.contains(&e.scene));
        }
        if let Some(origin) = &self.stack_origin
            && self.preset(origin).is_none()
        {
            self.stack_origin = None;
        }
    }
}

impl Sound {
    /// A new library sound with no overrides (everything inherits the project defaults).
    pub fn new(category: Category, name: String, file: FileInfo) -> Self {
        Self {
            id: new_id("snd"),
            category,
            name,
            file,
            volume: None,
            looping: None,
            fade_in_ms: None,
            fade_out_ms: None,
            tags: Vec::new(),
            favourite: false,
            source: None,
            last_used: None,
        }
    }

    /// The Library folder the sound's file is in: the path between `audio/<category>/` and
    /// the file name, e.g. "Barovia" or "Barovia/Castle". Empty for the category's own folder.
    pub fn folder(&self) -> String {
        folder_of_path(&self.file.path)
    }
}

/// See `Sound::folder`. Works on a stored path like `audio/music/Barovia/theme.ogg`.
pub fn folder_of_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    // audio / <category> / folders... / file
    if parts.len() <= 3 {
        return String::new();
    }
    parts[2..parts.len() - 1].join("/")
}

/// How the current stack relates to the saved presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StackStatus {
    Empty,
    /// Matches a preset exactly.
    Saved,
    /// Loaded from a preset (`stack_origin`) and changed since.
    Modified,
    /// Not from a preset and doesn't match one.
    Unsaved,
}

/// A new random id with a short prefix that says what it is, e.g. `snd-3f2a…`.
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_documented_example_and_fills_in_missing_fields() {
        // A trimmed version of the example in docs/save-format.md and docs/scenes.md.
        let json = r#"{
            "formatVersion": 1,
            "name": "Curse of Strahd",
            "masterVolume": 0.9,
            "categories": {
                "music": { "volume": 0.8 },
                "ambience": { "volume": 0.6 },
                "effects": { "volume": 1.0 }
            },
            "sounds": [{
                "id": "snd-birds", "category": "ambience", "name": "Birds",
                "file": { "path": "audio/ambience/birds.ogg", "size": 10, "modified": "2026-11-12T15:04:00Z" },
                "volume": 0.5
            }],
            "scenes": [{
                "id": "scn-forest", "name": "Forest", "music": null,
                "ambience": [{ "sound": "snd-birds", "volume": 0.8, "role": "background" }],
                "effects": []
            }],
            "defaults": {
                "duckLevel": 0.3, "transitionMs": 2500, "soundVolume": 1.0,
                "categories": {
                    "music":    { "loop": true,  "fadeInMs": 2000, "fadeOutMs": 2000, "shuffle": false },
                    "ambience": { "loop": true,  "fadeInMs": 1500, "fadeOutMs": 1500 },
                    "effects":  { "loop": false, "fadeInMs": 0,    "fadeOutMs": 0 }
                }
            }
        }"#;

        let project: Project = serde_json::from_str(json).expect("example should parse");

        assert_eq!(project.scenes[0].ambience[0].role, Some(Role::Background));
        assert!(project.stack.is_empty(), "missing `stack` becomes empty");
        assert_eq!(project.backups.keep, 1, "missing `backups` uses defaults");

        let birds = project.resolve_sound(&project.sounds[0]);
        assert_eq!(birds.volume, 0.5, "own value wins");
        assert!(birds.looping, "inherited from ambience defaults");
        assert_eq!(birds.fade_in_ms, 1500);
    }

    #[test]
    fn unset_overrides_are_not_written() {
        let mut project = Project::new("Test", ProjectDefaults::default());
        project.sounds.push(Sound {
            id: "snd-1".into(),
            category: Category::Effects,
            name: "Door".into(),
            file: FileInfo {
                path: "audio/effects/door.ogg".into(),
                size: 1,
                modified: "2026-11-12T15:04:00Z".into(),
                sha256: None,
                loudness: None,
            },
            volume: None,
            looping: Some(true),
            fade_in_ms: None,
            fade_out_ms: None,
            tags: Vec::new(),
            favourite: false,
            source: None,
            last_used: None,
        });

        let json = serde_json::to_string(&project).unwrap();
        assert!(json.contains(r#""loop":true"#));
        assert!(!json.contains("fadeInMs\":null"));
        assert!(!json.contains("sha256"));
    }

    fn sound_with_loudness(category: Category, lufs: f32, peak: f32) -> Sound {
        Sound {
            id: "snd".into(),
            category,
            name: "x".into(),
            file: FileInfo {
                path: "audio/x.ogg".into(),
                size: 1,
                modified: String::new(),
                sha256: None,
                loudness: Some(Loudness { lufs, peak }),
            },
            volume: None,
            looping: None,
            fade_in_ms: None,
            fade_out_ms: None,
            tags: Vec::new(),
            favourite: false,
            source: None,
            last_used: None,
        }
    }

    #[test]
    fn normalization_turns_loud_files_down_and_quiet_files_up() {
        let p = Project::new("Test", ProjectDefaults::default()); // target −18 LUFS
        let loud = sound_with_loudness(Category::Music, -10.0, 1.0);
        let quiet = sound_with_loudness(Category::Ambience, -24.0, 0.25);
        assert_eq!(p.normalization_gain_db(&loud), -8.0);
        assert_eq!(p.normalization_gain_db(&quiet), 6.0);
    }

    #[test]
    fn normalization_never_clips_or_boosts_too_much() {
        let p = Project::new("Test", ProjectDefaults::default());
        // Wants +6 dB, but its peak (0.71 ≈ −3 dBFS) only leaves 3 dB of headroom.
        let peaky = sound_with_loudness(Category::Music, -24.0, 0.7079);
        assert!((p.normalization_gain_db(&peaky) - 3.0).abs() < 0.01);
        // Wants +22 dB; capped.
        let very_quiet = sound_with_loudness(Category::Ambience, -40.0, 0.01);
        assert_eq!(p.normalization_gain_db(&very_quiet), MAX_NORMALIZATION_BOOST_DB);
    }

    #[test]
    fn normalization_respects_the_category_switches() {
        let mut p = Project::new("Test", ProjectDefaults::default());
        let effect = sound_with_loudness(Category::Effects, -10.0, 1.0);
        assert_eq!(p.normalization_gain_db(&effect), 0.0, "effects are off by default");
        p.defaults.normalize.music = false;
        let music = sound_with_loudness(Category::Music, -10.0, 1.0);
        assert_eq!(p.normalization_gain_db(&music), 0.0);
    }

    #[test]
    fn deleting_a_sound_removes_it_from_scenes() {
        let mut project = Project::new("Test", ProjectDefaults::default());
        project.scenes.push(Scene {
            id: "scn-1".into(),
            name: "Forest".into(),
            group: None,
            hotkey: None,
            music: Some(Playlist {
                tracks: vec!["gone".into()],
                shuffle: None,
            }),
            ambience: vec![Layer {
                sound: "gone".into(),
                volume: None,
                role: None,
            }],
            effects: vec!["gone".into()],
            duck_level: None,
            transition_ms: None,
        blocks_music: false,
        });
        project.stack.push(StackEntry {
            id: "stk-1".into(),
            scene: "no-such-scene".into(),
            enabled: true,
        });

        project.remove_dangling_references();

        let scene = &project.scenes[0];
        assert!(scene.music.as_ref().unwrap().tracks.is_empty());
        assert!(scene.ambience.is_empty());
        assert!(scene.effects.is_empty());
        assert!(project.stack.is_empty());
    }

    #[test]
    fn folder_is_the_path_between_category_and_file() {
        assert_eq!(folder_of_path("audio/music/theme.ogg"), "");
        assert_eq!(folder_of_path("audio/music/Barovia/theme.ogg"), "Barovia");
        assert_eq!(folder_of_path("audio/ambience/A/B/rain.ogg"), "A/B");
    }

    #[test]
    fn stack_status_follows_the_origin_preset() {
        let mut p = Project::new("Test", ProjectDefaults::default());
        assert_eq!(p.stack_status(), StackStatus::Empty);
        p.stack.push(StackEntry { id: "e1".into(), scene: "s1".into(), enabled: true });
        assert_eq!(p.stack_status(), StackStatus::Unsaved);

        p.stack_presets.push(StackPreset {
            id: "pre-1".into(),
            name: "P".into(),
            group: None,
            stack: vec![PresetEntry { scene: "s1".into(), enabled: true }],
        });
        assert_eq!(p.stack_status(), StackStatus::Saved, "matches a preset without an origin");

        p.stack_origin = Some("pre-1".into());
        p.stack[0].enabled = false;
        assert_eq!(p.stack_status(), StackStatus::Modified);

        p.stack_presets.clear();
        p.remove_dangling_references();
        assert_eq!(p.stack_origin, None, "a deleted origin preset is forgotten");
    }
}
