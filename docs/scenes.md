# Scenes

> Status: proposed ([decision 0008](decisions/0008-scenes.md)).

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

## Overlay scenes (undecided)

An overlay is applied on top of whatever scene is playing:

- It may **duck** (lower) the current ambience, for example to 30%, instead of replacing it.
- It replaces the music and adds its own layers.
- Turning it off restores the underlying scene exactly as it was.

One "Combat" overlay then works in a forest, a cave or a city.

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
    "name": "Goblin ambush",
    "group": "Wilderness",
    "hotkey": "2",
    "music":   { "tracks": ["snd-battle-drums"], "shuffle": false },
    "ambience": [
      { "sound": "snd-birds",  "volume": 0.2 },
      { "sound": "snd-wind",   "volume": 0.3 },
      { "sound": "snd-clash",  "volume": 0.8 },
      { "sound": "snd-goblins","volume": 0.6 }
    ],
    "effects": ["snd-screech", "snd-sword", "snd-arrow"],
    "transitionMs": 2500
  }
]
```
