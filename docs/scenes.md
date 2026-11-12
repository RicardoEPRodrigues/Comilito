# Scenes

> Status: accepted ([0008](decisions/0008-scenes.md), [0009](decisions/0009-overlay-scenes-and-roles.md)).

## Sounds vs. scenes

- **Sounds** are the project's library: an audio file plus its default settings (volume, loop,
  fades), in one of three categories: Music, Ambience, Effects.
- **Scenes** are saved mixes that say what plays together:
  - **Music:** a playlist (one or more tracks, optional shuffle).
  - **Ambience:** a set of layers, each with its own volume in this scene.
  - **Effects:** the sounds shown on the quick-access pad for this scene.

The same sound can appear in many scenes at different volumes.

## Switching scenes

Switching from scene A to scene B:

| Sound is in… | What happens |
|---|---|
| A and B | keeps playing, tweens to B's volume (never restarts) |
| A only | fades out, then stops |
| B only | starts and fades in |

Music crossfades if B's playlist doesn't contain the current track. Otherwise it continues.
Effects already playing are left to finish.

### Example

| Sound | Wilderness | Goblin ambush |
|---|---|---|
| 🎵 Peaceful forest | ▶ | — |
| 🎵 Battle drums | — | ▶ |
| 🌲 Forest birds | 80% | 20% |
| 🌲 Wind | 50% | 30% |
| 🌲 Stream | 40% | — |
| ⚔️ Battle clash | — | 80% |
| ⚔️ Goblin chatter | — | 60% |
| 💥 Effects pad | arrow, twig snap | goblin screech, sword hit, arrow |

## Overlay scenes

See [decision 0009](decisions/0009-overlay-scenes-and-roles.md). An overlay is applied on top of
whatever scene is playing, and turning it off restores that scene exactly as it was. One "Combat"
overlay then works in a forest, a cave or a city.

### Layer roles

Each ambience layer in a scene can have a role: `foreground`, `background` or `supporting`.
The role belongs to the layer in that scene, not to the sound.

While an overlay is active, each underlying layer is handled like this:

| Underlying layer | Effect |
|---|---|
| no role | ducked to the overlay's `duckLevel` (default 30%) |
| role R, and the overlay has a layer with role R | ducked (the overlay takes over that role) |
| role R, and the overlay has no layer with role R | plays unchanged |

### Music and effects

- If the overlay has music, the underlying music fades out and pauses, then resumes where it left
  off when the overlay ends. If the overlay has no music, the underlying music continues.
- The effects pad shows the overlay's effects first, then the scene's.

### Rules

- One overlay at a time (v1). Triggering another replaces it.
- Changing the base scene during an overlay crossfades the base underneath, and the overlay stays.

### Example: Combat overlay on Wilderness

| Layer | Wilderness | Combat overlay | Result |
|---|---|---|---|
| 🎵 Peaceful forest | music | 🎵 Battle drums | forest pauses, drums fade in |
| 🌲 Birds | background, 80% | (no background) | unchanged, 80% |
| 🌲 Wind | background, 50% | (no background) | unchanged, 50% |
| 🌲 Stream | supporting, 40% | ⚔️ Goblin chatter, supporting | stream ducked to 30% × 40% = 12% |
| ⚔️ Battle clash | — | foreground, 80% | fades in |

## UI

- **Scene list** (left): grouped (for example "Phandalin", "Session 3"). Click or press 1–9 to switch.
- **Now playing** (centre): what's playing, with a live slider for each sound.
- **Effects pad** (right): big buttons for the current scene's effects.
- **Mixer bar** (bottom): master and per-category volumes.
- **Play / Edit mode:** in Play mode the library and scenes can't be changed by accident.
- **Space:** stop all (with a quick fade).
- **Save current mix to scene:** stores the live tweaks.

## Data (draft, in `comilito.json`)

```json
"scenes": [
  {
    "id": "scn-1",
    "kind": "scene",
    "name": "Wilderness",
    "group": "Wilderness",
    "hotkey": "1",
    "music":   { "tracks": ["snd-peaceful-forest"], "shuffle": false },
    "ambience": [
      { "sound": "snd-birds",  "volume": 0.8, "role": "background" },
      { "sound": "snd-wind",   "volume": 0.5, "role": "background" },
      { "sound": "snd-stream", "volume": 0.4, "role": "supporting" }
    ],
    "effects": ["snd-twig", "snd-arrow"],
    "transitionMs": 2500
  },
  {
    "id": "scn-2",
    "kind": "overlay",
    "name": "Combat",
    "hotkey": "F1",
    "duckLevel": 0.3,
    "music":   { "tracks": ["snd-battle-drums"], "shuffle": false },
    "ambience": [
      { "sound": "snd-clash",   "volume": 0.8, "role": "foreground" },
      { "sound": "snd-goblins", "volume": 0.6, "role": "supporting" }
    ],
    "effects": ["snd-screech", "snd-sword"],
    "transitionMs": 1500
  }
]
```

- `kind`: `scene` or `overlay`. `duckLevel` applies only to overlays.
- `role`: optional; one of `foreground`, `background`, `supporting`.
