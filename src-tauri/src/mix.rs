//! The target mix: given the stack, what should be playing and how loud?
//! The rules are in docs/scenes.md ("The scene stack").
//!
//! This is a pure function (no audio, no files, no clock), so the tricky rules can be
//! unit tested directly. The audio engine then fades from whatever is playing to this.

use std::collections::HashMap;

use serde::Serialize;

use crate::project::{Project, Scene};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetMix {
    /// The stack entry whose music plays: the top-most enabled scene with music.
    pub music_entry: Option<String>,
    /// Ambience sound id → effective volume (after ducking). A sound appears once even if
    /// several scenes use it: the highest entry wins.
    pub ambience: HashMap<String, f32>,
    /// Effects pad: sound ids of every enabled scene, top scene first, without repeats.
    pub effects: Vec<String>,
    /// Stack entry id → effective volume of each of its scene's ambience layers (same order
    /// as `scene.ambience`), for the UI. Disabled entries are absent.
    pub layer_volumes: HashMap<String, Vec<f32>>,
}

/// Computes the target mix from the project's stack.
pub fn target_mix(project: &Project) -> TargetMix {
    // Pair each enabled entry with its scene, bottom to top. Entries whose scene is gone
    // are skipped (they're cleaned up elsewhere).
    // `filter_map` is filter + map in one: the closure returns `Some(x)` to keep `x`.
    let enabled: Vec<(&str, &Scene)> = project
        .stack
        .iter()
        .filter(|e| e.enabled)
        .filter_map(|e| project.scene(&e.scene).map(|s| (e.id.as_str(), s)))
        .collect();

    // Music: walk from the top (`.rev()`) and take the first scene that has any.
    let music_entry = enabled
        .iter()
        .rev()
        .find(|(_, scene)| scene.music.as_ref().is_some_and(|m| !m.tracks.is_empty()))
        .map(|(entry_id, _)| entry_id.to_string());

    let mut mix = TargetMix {
        music_entry,
        ..Default::default()
    };

    // Ambience, bottom to top, so a higher entry overwrites a lower one for the same sound.
    for (i, (entry_id, scene)) in enabled.iter().enumerate() {
        let scenes_above = &enabled[i + 1..];
        let mut volumes = Vec::with_capacity(scene.ambience.len());

        for layer in &scene.ambience {
            let base = layer
                .volume
                .or_else(|| project.sound(&layer.sound).and_then(|s| s.volume))
                .unwrap_or(project.defaults.sound_volume);

            // Multiply by the duck level of every scene above that ducks this layer.
            let ducking: f32 = scenes_above
                .iter()
                .filter(|(_, above)| match layer.role {
                    // No role: every scene above ducks it.
                    None => true,
                    // With a role: only scenes that have a layer with the same role.
                    Some(role) => above.ambience.iter().any(|l| l.role == Some(role)),
                })
                .map(|(_, above)| project.scene_duck_level(above))
                .product();

            let volume = base * ducking;
            volumes.push(volume);
            mix.ambience.insert(layer.sound.clone(), volume);
        }
        mix.layer_volumes.insert(entry_id.to_string(), volumes);
    }

    // Effects, top scene first, each sound once.
    for (_, scene) in enabled.iter().rev() {
        for id in &scene.effects {
            if !mix.effects.contains(id) {
                mix.effects.push(id.clone());
            }
        }
    }

    mix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Layer, Playlist, ProjectDefaults, Role, StackEntry};

    // Small builders so each test reads like the tables in docs/scenes.md.

    fn layer(sound: &str, volume: f32, role: Option<Role>) -> Layer {
        Layer {
            sound: sound.into(),
            volume: Some(volume),
            role,
        }
    }

    fn scene(id: &str, music: &[&str], ambience: Vec<Layer>, effects: &[&str]) -> Scene {
        Scene {
            id: id.into(),
            name: id.into(),
            group: None,
            hotkey: None,
            music: if music.is_empty() {
                None
            } else {
                Some(Playlist {
                    tracks: music.iter().map(|s| s.to_string()).collect(),
                    shuffle: None,
                })
            },
            ambience,
            effects: effects.iter().map(|s| s.to_string()).collect(),
            duck_level: None,
            transition_ms: None,
        }
    }

    /// The three scenes from the example in docs/scenes.md.
    fn example_project() -> Project {
        let mut p = Project::new("Test", ProjectDefaults::default());
        p.scenes = vec![
            scene("music", &["travel"], vec![], &[]),
            scene(
                "forest",
                &[],
                vec![
                    layer("birds", 0.8, Some(Role::Background)),
                    layer("wind", 0.5, Some(Role::Background)),
                    layer("stream", 0.4, None),
                ],
                &["twig"],
            ),
            scene(
                "combat",
                &["drums"],
                vec![
                    layer("clash", 0.8, Some(Role::Foreground)),
                    layer("goblins", 0.6, Some(Role::Supporting)),
                ],
                &["screech", "twig"],
            ),
        ];
        p
    }

    /// Sets the stack, bottom to top. `(scene id, enabled)`.
    fn set_stack(p: &mut Project, entries: &[(&str, bool)]) {
        p.stack = entries
            .iter()
            .enumerate()
            .map(|(i, (scene, enabled))| StackEntry {
                id: format!("e{i}"),
                scene: scene.to_string(),
                enabled: *enabled,
            })
            .collect();
    }

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn empty_stack_plays_nothing() {
        let p = example_project();
        assert_eq!(target_mix(&p), TargetMix::default());
    }

    #[test]
    fn documented_example_music_and_forest() {
        let mut p = example_project();
        set_stack(&mut p, &[("music", true), ("forest", true)]);
        let mix = target_mix(&p);

        assert_eq!(mix.music_entry.as_deref(), Some("e0"));
        assert!(approx(mix.ambience["birds"], 0.8));
        assert!(approx(mix.ambience["wind"], 0.5));
        assert!(approx(mix.ambience["stream"], 0.4));
    }

    #[test]
    fn documented_example_with_combat_on_top() {
        let mut p = example_project();
        set_stack(&mut p, &[("music", true), ("forest", true), ("combat", true)]);
        let mix = target_mix(&p);

        // Combat's music covers the travelling music.
        assert_eq!(mix.music_entry.as_deref(), Some("e2"));
        // Combat has no background layer, so background layers aren't ducked.
        assert!(approx(mix.ambience["birds"], 0.8));
        assert!(approx(mix.ambience["wind"], 0.5));
        // No role: ducked to 30%.
        assert!(approx(mix.ambience["stream"], 0.4 * 0.3));
        assert!(approx(mix.ambience["clash"], 0.8));
        assert!(approx(mix.ambience["goblins"], 0.6));
        // Effects: top scene first, no repeats.
        assert_eq!(mix.effects, vec!["screech", "twig"]);
    }

    #[test]
    fn disabling_the_top_scene_restores_the_mix_below() {
        let mut p = example_project();
        set_stack(&mut p, &[("music", true), ("forest", true), ("combat", false)]);
        let mix = target_mix(&p);

        assert_eq!(mix.music_entry.as_deref(), Some("e0"));
        assert!(approx(mix.ambience["stream"], 0.4));
        assert!(!mix.ambience.contains_key("clash"));
        assert!(!mix.layer_volumes.contains_key("e2"));
    }

    #[test]
    fn ducking_multiplies_and_uses_each_scenes_own_level() {
        let mut p = example_project();
        // Two more scenes above the forest: one at the default 0.3, one at 0.5.
        p.scenes.push(scene("rain", &[], vec![layer("drops", 1.0, None)], &[]));
        let mut quiet = scene("quiet", &[], vec![], &[]);
        quiet.duck_level = Some(0.5);
        p.scenes.push(quiet);

        set_stack(&mut p, &[("forest", true), ("rain", true), ("quiet", true)]);
        let mix = target_mix(&p);

        assert!(approx(mix.ambience["stream"], 0.4 * 0.3 * 0.5));
        assert!(approx(mix.ambience["drops"], 1.0 * 0.5));
    }

    #[test]
    fn a_role_is_ducked_only_when_a_scene_above_takes_it_over() {
        let mut p = example_project();
        p.scenes.push(scene(
            "storm",
            &[],
            vec![layer("thunder", 1.0, Some(Role::Background))],
            &[],
        ));
        set_stack(&mut p, &[("forest", true), ("storm", true)]);
        let mix = target_mix(&p);

        assert!(approx(mix.ambience["birds"], 0.8 * 0.3));
        assert!(approx(mix.ambience["thunder"], 1.0));
    }

    #[test]
    fn same_sound_in_two_entries_uses_the_highest() {
        let mut p = example_project();
        // The forest twice: the lower copy is ducked, the upper one isn't, and the upper wins.
        set_stack(&mut p, &[("forest", true), ("forest", true)]);
        let mix = target_mix(&p);

        assert!(approx(mix.ambience["stream"], 0.4));
        assert!(approx(mix.layer_volumes["e0"][2], 0.4 * 0.3));
        assert_eq!(mix.ambience.len(), 3);
    }

    #[test]
    fn layer_without_volume_uses_the_sound_then_the_default() {
        let mut p = example_project();
        p.scenes[1].ambience[2].volume = None; // stream
        p.defaults.sound_volume = 0.7;
        set_stack(&mut p, &[("forest", true)]);

        assert!(approx(target_mix(&p).ambience["stream"], 0.7));
    }
}
