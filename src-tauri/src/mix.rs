//! The target mix: given the stack, what should be playing and how loud?
//! The rules are in docs/scenes.md ("The scene stack").
//!
//! This is a pure function (no audio, no files, no clock), so the tricky rules can be
//! unit tested directly. The audio engine then fades from whatever is playing to this.

use std::collections::HashMap;

use serde::Serialize;

use crate::project::{PerCategory, Project, Scene};

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
    /// The scene whose category volumes are used: the top-most enabled one. `None` when no
    /// scene is enabled; the project's own category volumes are used then (decision 0028).
    pub volumes_scene: Option<String>,
    /// Music, ambience and effects track volumes.
    pub volumes: PerCategory<f32>,
    /// What the video window shows (decision 0035): the top-most enabled scene with videos.
    /// `None` (a black screen) when there's none, or a scene above hides it.
    pub video: Option<VideoTarget>,
    /// Video effects pad: video ids of every enabled scene, top scene first, without repeats.
    pub video_effects: Vec<String>,
}

/// A scene's video playlist with every inherited setting filled in.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoTarget {
    pub entry_id: String,
    pub scene_id: String,
    pub videos: Vec<String>,
    pub shuffle: bool,
    pub switch_minutes: u32,
    pub resume: bool,
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

    // Music: walk from the top (`.rev()`) and stop at the first scene that has music or
    // blocks it. If that scene blocks music without having any, nothing plays.
    let has_music = |scene: &Scene| scene.music.as_ref().is_some_and(|m| !m.tracks.is_empty());
    let music_entry = enabled
        .iter()
        .rev()
        .find(|(_, scene)| has_music(scene) || scene.blocks_music)
        .filter(|(_, scene)| has_music(scene))
        .map(|(entry_id, _)| entry_id.to_string());

    // Category volumes: the top-most enabled scene decides them for every sound playing,
    // including music coming from a scene below it.
    let top = enabled.last().map(|(_, scene)| *scene);

    let mut mix = TargetMix {
        music_entry,
        volumes_scene: top.map(|scene| scene.id.clone()),
        volumes: match top {
            Some(scene) => project.scene_volumes(scene),
            None => project.default_volumes(),
        },
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
                // Only scenes that have a layer with the same role take it over, or that duck
                // everything below (`ducks_all`).
                .filter(|(_, above)| above.ducks_all || above.ambience.iter().any(|l| l.role == layer.role))
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
        for id in &scene.video_effects {
            if !mix.video_effects.contains(id) {
                mix.video_effects.push(id.clone());
            }
        }
    }

    // Video: the same rule as music. The top-most scene with videos shows them; a scene above
    // without videos lets them through, unless it hides them.
    let has_videos = |scene: &Scene| scene.video.as_ref().is_some_and(|v| !v.videos.is_empty());
    let d = &project.defaults.video;
    mix.video = enabled
        .iter()
        .rev()
        .find(|(_, scene)| has_videos(scene) || scene.hides_video)
        .and_then(|(entry_id, scene)| {
            let playlist = scene.video.as_ref().filter(|v| !v.videos.is_empty())?;
            Some(VideoTarget {
                entry_id: entry_id.to_string(),
                scene_id: scene.id.clone(),
                videos: playlist.videos.clone(),
                shuffle: playlist.shuffle.unwrap_or(d.shuffle),
                switch_minutes: playlist.switch_minutes.unwrap_or(d.switch_minutes),
                resume: playlist.resume.unwrap_or(d.resume),
            })
        });

    mix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Layer, Playlist, ProjectDefaults, Role, StackEntry};

    // Small builders so each test reads like the tables in docs/scenes.md.

    fn layer(sound: &str, volume: f32, role: Role) -> Layer {
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
        blocks_music: false,
        ducks_all: false,
        volumes: None,
        video: None,
        hides_video: false,
        video_effects: Vec::new(),
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
                    layer("birds", 0.8, Role::Background),
                    layer("wind", 0.5, Role::Background),
                    layer("stream", 0.4, Role::Foreground),
                ],
                &["twig"],
            ),
            scene(
                "combat",
                &["drums"],
                vec![
                    layer("clash", 0.8, Role::Foreground),
                    layer("goblins", 0.6, Role::Supporting),
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
    fn the_top_most_scene_with_videos_shows_them() {
        let mut p = example_project();
        p.scenes[0].video = Some(crate::project::VideoPlaylist { videos: vec!["dusk".into()], ..Default::default() });
        p.scenes[2].video = Some(crate::project::VideoPlaylist {
            videos: vec!["fire".into()],
            switch_minutes: Some(2),
            ..Default::default()
        });
        p.scenes[1].video_effects = vec!["flash".into()];
        set_stack(&mut p, &[("music", true), ("forest", true)]);

        // Forest has no videos: Music's show through, with the project's defaults.
        let mix = target_mix(&p);
        let video = mix.video.unwrap();
        assert_eq!((video.scene_id.as_str(), video.videos.as_slice()), ("music", &["dusk".to_string()][..]));
        assert_eq!((video.switch_minutes, video.resume, video.shuffle), (10, true, false));
        assert_eq!(mix.video_effects, ["flash"]);

        // Combat on top has its own, with its own switch interval.
        set_stack(&mut p, &[("music", true), ("forest", true), ("combat", true)]);
        let video = target_mix(&p).video.unwrap();
        assert_eq!(video.scene_id, "combat");
        assert_eq!(video.switch_minutes, 2);

        // A disabled scene doesn't count.
        set_stack(&mut p, &[("music", true), ("combat", false)]);
        assert_eq!(target_mix(&p).video.unwrap().scene_id, "music");
    }

    #[test]
    fn a_scene_can_hide_the_video_below() {
        let mut p = example_project();
        p.scenes[0].video = Some(crate::project::VideoPlaylist { videos: vec!["dusk".into()], ..Default::default() });
        p.scenes[1].hides_video = true;
        set_stack(&mut p, &[("music", true), ("forest", true)]);
        assert_eq!(target_mix(&p).video, None);
        set_stack(&mut p, &[("forest", true), ("music", true)]);
        assert!(target_mix(&p).video.is_some(), "only scenes above hide it");
    }

    #[test]
    fn empty_stack_plays_nothing() {
        let p = example_project();
        let mix = target_mix(&p);
        assert_eq!(mix.music_entry, None);
        assert!(mix.ambience.is_empty());
        assert!(mix.effects.is_empty());
        assert_eq!(mix.volumes_scene, None);
        assert_eq!(mix.volumes, p.default_volumes());
    }

    #[test]
    fn the_top_enabled_scene_sets_the_category_volumes() {
        let mut p = example_project();
        let forest_volumes = PerCategory {
            music: 0.2,
            ambience: 0.9,
            effects: 1.0,
        };
        p.scenes[1].volumes = Some(forest_volumes.clone());

        // The forest is on top: its volumes apply, also to the music from the scene below.
        set_stack(&mut p, &[("music", true), ("forest", true)]);
        let mix = target_mix(&p);
        assert_eq!(mix.volumes_scene.as_deref(), Some("forest"));
        assert_eq!(mix.volumes, forest_volumes);

        // Combat on top has no volumes of its own: the project's are used.
        set_stack(&mut p, &[("music", true), ("forest", true), ("combat", true)]);
        let mix = target_mix(&p);
        assert_eq!(mix.volumes_scene.as_deref(), Some("combat"));
        assert_eq!(mix.volumes, p.default_volumes());

        // A disabled scene on top doesn't count.
        set_stack(&mut p, &[("music", true), ("forest", true), ("combat", false)]);
        assert_eq!(target_mix(&p).volumes, forest_volumes);
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
        // Combat has a foreground layer, so it takes over the stream's role: ducked to 30%.
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
    fn a_music_blocking_scene_silences_the_music_below() {
        let mut p = example_project();
        p.scenes[1].blocks_music = true; // the forest
        set_stack(&mut p, &[("music", true), ("forest", true)]);
        assert_eq!(target_mix(&p).music_entry, None);

        // Music above the blocker still plays.
        set_stack(&mut p, &[("music", true), ("forest", true), ("combat", true)]);
        assert_eq!(target_mix(&p).music_entry.as_deref(), Some("e2"));

        // A disabled blocker doesn't block.
        set_stack(&mut p, &[("music", true), ("forest", false)]);
        assert_eq!(target_mix(&p).music_entry.as_deref(), Some("e0"));
    }

    #[test]
    fn ducking_multiplies_and_uses_each_scenes_own_level() {
        let mut p = example_project();
        // Two more scenes above the forest: one at the default 0.3, one at 0.5.
        p.scenes.push(scene("rain", &[], vec![layer("drops", 1.0, Role::Foreground)], &[]));
        let mut quiet = scene("quiet", &[], vec![layer("hush", 0.2, Role::Foreground)], &[]);
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
            vec![layer("thunder", 1.0, Role::Background)],
            &[],
        ));
        set_stack(&mut p, &[("forest", true), ("storm", true)]);
        let mix = target_mix(&p);

        assert!(approx(mix.ambience["birds"], 0.8 * 0.3));
        assert!(approx(mix.ambience["thunder"], 1.0));
    }

    #[test]
    fn a_scene_without_ambience_ducks_nothing() {
        let mut p = example_project();
        // Travelling music on top of the forest: no layer takes over any role.
        set_stack(&mut p, &[("forest", true), ("music", true)]);
        let mix = target_mix(&p);

        assert!(approx(mix.ambience["stream"], 0.4));
        assert!(approx(mix.ambience["birds"], 0.8));
    }

    #[test]
    fn a_scene_that_ducks_all_ducks_every_role() {
        let mut p = example_project();
        p.scenes[0].ducks_all = true; // travelling music, which has no ambience
        set_stack(&mut p, &[("forest", true), ("music", true)]);
        let mix = target_mix(&p);

        assert!(approx(mix.ambience["stream"], 0.4 * 0.3));
        assert!(approx(mix.ambience["birds"], 0.8 * 0.3));
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
