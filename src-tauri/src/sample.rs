//! The sample project offered when creating a new project: a few bundled sounds
//! (src-tauri/resources/sample, credited in CREDITS.md) and scenes that show off the stack.
//!
//! The audio ships inside the app as a Tauri "resource" (see `bundle.resources` in
//! tauri.conf.json). Here we only get the folder it was installed to, so this module stays
//! free of Tauri and can be tested with a fake folder.

use std::path::Path;

use crate::{
    project::{Category, Layer, Playlist, PresetEntry, Project, Role, Scene, Sound, StackEntry, StackPreset, new_id},
    storage,
};

/// The bundled sounds: (file inside the sample folder, category, display name).
const SOUNDS: [(&str, Category, &str); 4] = [
    ("music/chopin-nocturne-op9-no2.mp3", Category::Music, "Chopin – Nocturne Op. 9 No. 2"),
    ("ambience/heavy-rain.mp3", Category::Ambience, "Heavy rain"),
    ("ambience/civil-war-drummer.mp3", Category::Ambience, "War drummer"),
    ("effects/tolling-bell.mp3", Category::Effects, "Tolling bell"),
];

/// Copies the sample audio into the project folder `dir` and adds sounds, scenes, a stack and
/// a preset to `project`.
pub fn fill_sample_project(dir: &Path, sample_dir: &Path, project: &mut Project) -> Result<(), String> {
    // Copy every file and remember the new sound ids, in the same order as SOUNDS.
    let mut ids = Vec::new();
    for (file, category, name) in SOUNDS {
        let source = sample_dir.join(file);
        let info = storage::copy_audio_into_project(dir, category, &source)
            .map_err(|e| format!("could not copy the sample audio: {e}"))?;
        let id = new_id("snd");
        project.sounds.push(Sound {
            id: id.clone(),
            category,
            name: name.to_string(),
            file: info,
            volume: None,
            looping: None,
            fade_in_ms: None,
            fade_out_ms: None,
        });
        ids.push(id);
    }
    // Destructure the Vec into named variables. `let [a, b] = ... else` only works if the
    // length matches, which it always does here (one id per entry in SOUNDS).
    let [chopin, rain, drummer, bell] = ids.as_slice() else {
        unreachable!("one id per sample sound");
    };

    let scene = |name: &str, hotkey: &str| Scene {
        id: new_id("scn"),
        name: name.to_string(),
        group: Some("Sample".to_string()),
        hotkey: Some(hotkey.to_string()),
        music: None,
        ambience: Vec::new(),
        effects: Vec::new(),
        duck_level: None,
        transition_ms: None,
    };

    // Music only, at the bottom of the stack.
    let mut nocturne = scene("Nocturne", "1");
    nocturne.music = Some(Playlist {
        tracks: vec![chopin.clone()],
        shuffle: None,
    });

    // Rain as a foreground layer: a scene above with its own foreground layer ducks it.
    let mut rainy = scene("Rainy night", "2");
    rainy.ambience = vec![Layer {
        sound: rain.clone(),
        volume: Some(0.8),
        role: Some(Role::Foreground),
    }];
    rainy.effects = vec![bell.clone()];

    // The drummer takes over the foreground, so the rain below drops to the duck level.
    let mut combat = scene("Combat", "3");
    combat.ambience = vec![Layer {
        sound: drummer.clone(),
        volume: Some(0.9),
        role: Some(Role::Foreground),
    }];
    combat.effects = vec![bell.clone()];
    combat.transition_ms = Some(1500);

    // Start with music and rain in the stack, ready to press Space.
    project.stack = [&nocturne, &rainy]
        .iter()
        .map(|s| StackEntry {
            id: new_id("stk"),
            scene: s.id.clone(),
            enabled: true,
        })
        .collect();
    project.stack_presets.push(StackPreset {
        id: new_id("pre"),
        name: "Battle in the rain".to_string(),
        stack: [&nocturne, &rainy, &combat]
            .iter()
            .map(|s| PresetEntry {
                scene: s.id.clone(),
                enabled: true,
            })
            .collect(),
    });
    project.scenes.extend([nocturne, rainy, combat]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mix::target_mix, project::ProjectDefaults, storage::test_support::TempDir};

    #[test]
    fn builds_a_playable_sample_from_the_bundled_files() {
        let root = TempDir::new();
        // A fake sample folder with the same layout as the real one.
        let sample = root.0.join("sample");
        for (file, _, _) in SOUNDS {
            let path = sample.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"fake audio").unwrap();
        }
        let dir = storage::create_project(&root.0, "Sample", ProjectDefaults::default()).unwrap();
        let mut project = storage::load_project(&dir).unwrap();

        fill_sample_project(&dir, &sample, &mut project).unwrap();

        assert_eq!(project.sounds.len(), 4);
        assert!(storage::missing_sounds(&dir, &project).is_empty());
        assert_eq!(project.scenes.len(), 3);
        assert_eq!(project.stack.len(), 2);
        assert_eq!(project.stack_presets[0].stack.len(), 3);

        // The starting stack plays the nocturne and the rain at full layer volume.
        let mix = target_mix(&project);
        assert!(mix.music_entry.is_some());
        assert_eq!(mix.ambience.len(), 1);
    }

    #[test]
    fn missing_sample_folder_is_an_error() {
        let root = TempDir::new();
        let dir = storage::create_project(&root.0, "Sample", ProjectDefaults::default()).unwrap();
        let mut project = storage::load_project(&dir).unwrap();
        assert!(fill_sample_project(&dir, &root.0.join("nope"), &mut project).is_err());
    }
}
