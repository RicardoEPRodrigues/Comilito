//! App preferences: settings for this computer, stored in `preferences.json` in the OS
//! config folder (docs/preferences.md, decision 0012).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    project::{BackupSettings, CategoryMix, PerCategory, Project, ProjectDefaults},
    storage,
};

const PREFS_FILE: &str = "preferences.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub format_version: u32,
    /// Where projects live. `None` means the default, `~/Documents/Comilito`.
    #[serde(default)]
    pub projects_folder: Option<PathBuf>,
    #[serde(default)]
    pub playback: Playback,
    #[serde(default)]
    pub behaviour: Behaviour,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub new_project_defaults: NewProjectDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playback {
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
}

fn ask() -> String {
    "ask".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    /// "system", "light" or "dark".
    pub theme: String,
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
            playback: Playback::default(),
            behaviour: Behaviour::default(),
            appearance: Appearance::default(),
            new_project_defaults: NewProjectDefaults::default(),
        }
    }
}

impl Default for Playback {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "system".into(),
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
    std::fs::read_to_string(config_dir.join(PREFS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
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
