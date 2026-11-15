//! App preferences: settings for this computer, stored in `preferences.json` in the OS
//! config folder (docs/preferences.md, decision 0012).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    project::{BackupSettings, CategoryMix, PerCategory, Project, ProjectDefaults},
    storage,
};

const PREFS_FILE: &str = "preferences.json";
const RECENT_PROJECTS: usize = 20;

impl Preferences {
    /// Puts `dir` first in the recent projects.
    pub fn note_opened(&mut self, dir: &Path) {
        self.recent_projects.retain(|p| p != dir);
        self.recent_projects.insert(0, dir.to_path_buf());
        self.recent_projects.truncate(RECENT_PROJECTS);
    }

    /// After project folders moved from `from` to `to`, points the recent list at the new places.
    pub fn moved_projects(&mut self, from: &Path, to: &Path) {
        for p in &mut self.recent_projects {
            if let Ok(rest) = p.strip_prefix(from) {
                *p = to.join(rest);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub format_version: u32,
    /// Where projects live. `None` means the default, `~/Documents/Comilito`.
    #[serde(default)]
    pub projects_folder: Option<PathBuf>,
    /// Open the most recently used project when Comilito starts, instead of the project list.
    #[serde(default)]
    pub open_last_project: bool,
    /// Projects opened on this computer, most recent first (at most `RECENT_PROJECTS`).
    /// Written only by Rust (`AppState::open_project`); `set_preferences` keeps its own copy,
    /// so a Preferences window opened earlier can't put an old list back.
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
    #[serde(default)]
    pub playback: Playback,
    #[serde(default)]
    pub behaviour: Behaviour,
    #[serde(default)]
    pub shortcuts: Shortcuts,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub video: VideoPrefs,
    #[serde(default)]
    pub new_project_defaults: NewProjectDefaults,
}

/// Video on this computer (decision 0035). The project's own video settings (playlists,
/// switching) live in the project; these are about the screen and the files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VideoPrefs {
    /// The on/off switch for the whole feature. Off hides everything video; projects keep
    /// their video data.
    pub enabled: bool,
    /// The screen the video window opens on (its name, as the OS gives it); `None` or a
    /// screen that isn't connected: the main screen.
    pub screen: Option<String>,
    pub fullscreen: bool,
    /// "cover" fills the window, cropping the edges; "contain" shows the whole picture.
    pub fit: String,
    pub crossfade_ms: u32,
    /// Downloads are limited to this height in pixels (720, 1080, 1440); 0 means the best
    /// the site has.
    pub download_height: u32,
    /// Videos that may not play: "ask", "always" (convert) or "never".
    pub convert: String,
}

impl Default for VideoPrefs {
    fn default() -> Self {
        Self {
            enabled: true,
            screen: None,
            fullscreen: false,
            fit: "cover".into(),
            crossfade_ms: 1000,
            download_height: 1080,
            convert: ask(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
    /// The audio output to play through (an `OutputDevice` id); `None` follows the system
    /// default (decision 0032).
    #[serde(default)]
    pub output_device: Option<String>,
    pub pause_fade_ms: u32,
    pub stop_fade_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Behaviour {
    /// "ask", "move" or "duplicate". Handled by the UI, which asks the question.
    pub duplicate_in_stack: String,
    /// "ask" or "trash". Handled by the UI.
    pub remove_sound: String,
    /// Closing an editor with unsaved changes: "ask", "save" or "discard". Handled by the UI.
    /// `default = ...` names a function that gives the value when the field is missing, so
    /// preferences saved before this setting existed still load (instead of failing and
    /// falling back to all defaults).
    #[serde(default = "ask")]
    pub unsaved_changes: String,
    /// Loading a preset over a tapestry that isn't saved in a preset: "ask" or "replace".
    #[serde(default = "ask")]
    pub replace_unsaved_tapestry: String,
    /// Ask before quitting while something would be cut off: downloads, fingerprinting,
    /// a backup or moving the projects folder.
    #[serde(default = "yes")]
    pub warn_on_quit: bool,
}

fn ask() -> String {
    "ask".into()
}

fn yes() -> bool {
    true
}

/// Keys that can be changed. Each is a `KeyboardEvent.key` value ("Backspace", "Delete", "x");
/// an empty string turns the shortcut off. Handled by the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Shortcuts {
    /// Removes the top-most scene from the tapestry.
    pub remove_top_scene: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            remove_top_scene: "Backspace".into(),
        }
    }
}

/// Themes, accent colour and decoration (decision 0016, docs/preferences.md).
/// `#[serde(default)]` on the struct fills any missing field from `Appearance::default()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Appearance {
    /// "system": follow the desktop's light/dark (`light_theme` / `dark_theme`);
    /// "fixed": always `theme`.
    pub mode: String,
    pub theme: String,
    pub light_theme: String,
    pub dark_theme: String,
    /// "theme", "desktop" or "custom".
    pub accent: String,
    pub custom_accent: String,
    pub capitals: bool,
    pub watermark: bool,
    pub rules: bool,
    pub blackletter: bool,
}

/// What a new project starts with. Changing these never touches existing projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProjectDefaults {
    pub master_volume: f32,
    pub categories: PerCategory<CategoryMix>,
    pub defaults: ProjectDefaults,
    pub backups: BackupSettings,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            format_version: 1,
            projects_folder: None,
            open_last_project: false,
            recent_projects: Vec::new(),
            playback: Playback::default(),
            behaviour: Behaviour::default(),
            shortcuts: Shortcuts::default(),
            appearance: Appearance::default(),
            video: VideoPrefs::default(),
            new_project_defaults: NewProjectDefaults::default(),
        }
    }
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            output_device: None,
            pause_fade_ms: 300,
            stop_fade_ms: 1000,
        }
    }
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            duplicate_in_stack: "ask".into(),
            remove_sound: "ask".into(),
            unsaved_changes: ask(),
            replace_unsaved_tapestry: ask(),
            warn_on_quit: true,
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            mode: "system".into(),
            theme: "codex".into(),
            light_theme: "codex".into(),
            dark_theme: "ironwood".into(),
            accent: "theme".into(),
            custom_accent: "#7a4fb5".into(),
            capitals: true,
            watermark: true,
            rules: true,
            blackletter: true,
        }
    }
}

impl Appearance {
    /// Preferences saved before themes existed had `theme: "system" | "light" | "dark"`.
    fn migrate(&mut self) {
        match self.theme.as_str() {
            "system" => {
                self.mode = "system".into();
                self.theme = "codex".into();
            }
            "light" => {
                self.mode = "fixed".into();
                self.theme = "codex".into();
            }
            "dark" => {
                self.mode = "fixed".into();
                self.theme = "ironwood".into();
            }
            _ => {}
        }
    }
}

impl Default for NewProjectDefaults {
    fn default() -> Self {
        // Reuse the values a fresh `Project` already has, so they're defined in one place.
        let p = Project::new("", ProjectDefaults::default());
        Self {
            master_volume: p.master_volume,
            categories: p.categories,
            defaults: p.defaults,
            backups: p.backups,
        }
    }
}

impl NewProjectDefaults {
    /// Copies these defaults into a project (new, or "Reset to my app defaults").
    pub fn apply_to(&self, project: &mut Project) {
        project.master_volume = self.master_volume;
        project.categories = self.categories.clone();
        project.defaults = self.defaults.clone();
        project.backups = self.backups.clone();
    }
}

/// Loads preferences from `config_dir`, or returns defaults if there are none yet
/// (or the file is unreadable: preferences are never worth refusing to start over).
pub fn load(config_dir: &Path) -> Preferences {
    let mut prefs: Preferences = std::fs::read_to_string(config_dir.join(PREFS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    prefs.appearance.migrate();
    prefs
}

pub fn save(config_dir: &Path, prefs: &Preferences) -> Result<(), String> {
    std::fs::create_dir_all(config_dir)
        .map_err(|e| format!("could not create {}: {e}", config_dir.display()))?;
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    storage::write_atomically(&config_dir.join(PREFS_FILE), json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::TempDir;

    #[test]
    fn older_preferences_without_new_fields_still_load() {
        let dir = TempDir::new();
        let old = r#"{"formatVersion":1,"playback":{"pauseFadeMs":300,"stopFadeMs":2500},
            "behaviour":{"duplicateInStack":"move","removeSound":"ask"}}"#;
        std::fs::write(dir.0.join(PREFS_FILE), old).unwrap();

        let prefs = load(&dir.0);
        assert_eq!(prefs.playback.stop_fade_ms, 2500, "kept, not reset to defaults");
        assert_eq!(prefs.behaviour.duplicate_in_stack, "move");
        assert_eq!(prefs.behaviour.unsaved_changes, "ask");
        assert!(prefs.behaviour.warn_on_quit);
        assert!(prefs.video.enabled, "video is on unless turned off");
        assert_eq!(prefs.video.download_height, 1080);
        assert_eq!(prefs.shortcuts.remove_top_scene, "Backspace");
        assert_eq!(prefs.playback.output_device, None, "the system default");
    }

    #[test]
    fn old_theme_setting_is_migrated() {
        let dir = TempDir::new();
        std::fs::write(dir.0.join(PREFS_FILE), r#"{"formatVersion":1,"appearance":{"theme":"dark"}}"#).unwrap();
        let prefs = load(&dir.0);
        assert_eq!(prefs.appearance.mode, "fixed");
        assert_eq!(prefs.appearance.theme, "ironwood");
        assert!(prefs.appearance.capitals, "new fields get their defaults");
    }

    #[test]
    fn missing_file_gives_defaults_and_saving_round_trips() {
        let dir = TempDir::new();
        let mut prefs = load(&dir.0);
        assert_eq!(prefs.playback.stop_fade_ms, 1000);

        prefs.playback.stop_fade_ms = 2000;
        save(&dir.0, &prefs).unwrap();
        assert_eq!(load(&dir.0).playback.stop_fade_ms, 2000);
    }
}
