# Scenes

> Status: accepted ([0008](decisions/0008-scenes.md), [0010](decisions/0010-scene-stack.md)).

## Sounds vs. scenes

- **Sounds** are the project's library: an audio file plus its default settings (volume, loop,
  fades), in one of three categories: Music, Ambience, Effects.
- **Scenes** are saved mixes that say what plays together:
  - **Music:** an optional playlist (one or more tracks, optional shuffle).
  - **Ambience:** an optional set of layers, each with its own volume and optional role in this scene.
  - **Effects:** the sounds shown on the quick-access pad for this scene.

The same sound can appear in many scenes at different volumes. A scene may contain only music,
only ambience, or both.

## The scene stack

What plays is decided by a **stack of scenes**. Any scene can go anywhere in the stack, and
there's no limit on its depth.

```
  ┌───────────────────┐
  │ Combat            │  ← top: drums, battle clash, goblin chatter
  ├───────────────────┤
  │ Forest            │  ← birds, wind, stream (no music)
  ├───────────────────┤
  │ Travelling music  │  ← bottom: music only
  └───────────────────┘
```

### Music

Only the **top-most scene that has music** plays its music. Music from lower scenes fades out and
**pauses**, and resumes where it left off when the scenes above it are removed. Music is never ducked.

### Ambience and ducking

Every scene added on top **ducks** the ambience of the scenes below it, lowering it by that scene's
`duck` amount (default **30%**, so a layer plays at 70% of its volume). Ducking stacks: a layer with
two ducking scenes above it plays at 0.7 × 0.7 = 49%.

**Roles** let a layer escape ducking. Each ambience layer can have a role: `foreground`,
`background` or `supporting`. For each scene above a layer:

| Layer below | Scene above… | Effect |
|---|---|---|
| no role | (any) | ducked |
| role R | has a layer with role R | ducked (that role is taken over) |
| role R | has no layer with role R | not ducked by this scene |

### Effects pad

Shows the effects of every scene in the stack, top scene first.

### Same sound in several scenes

A sound plays only once. If several scenes in the stack contain it, the **highest** scene's
volume (after its own ducking) is used.

### Example

| Sound | Stack: Music + Forest | + Combat pushed |
|---|---|---|
| 🎵 Travelling theme (Music) | ▶ playing | ⏸ paused |
| 🎵 Battle drums (Combat) | — | ▶ fades in |
| 🌲 Birds (Forest, background, 80%) | 80% | 80%: Combat has no background layer |
| 🌲 Wind (Forest, background, 50%) | 50% | 50% |
| 🌲 Stream (Forest, no role, 40%) | 40% | 28% (ducked by 30%) |
| ⚔️ Battle clash (Combat, foreground, 80%) | — | 80% |
| ⚔️ Goblin chatter (Combat, supporting, 60%) | — | 60% |

Removing Combat restores the left column: drums fade out, the travelling theme resumes and the stream returns to 40%.

## Stack operations

| Action | How | Result |
|---|---|---|
| **Switch** | click a scene / press its hotkey | the stack becomes just this scene |
| **Push** | Shift+click / Shift+hotkey | the scene is added on top |
| **Remove** | × on a scene in the stack bar | that scene is removed, wherever it is |
| **Pop** | Backspace | the top scene is removed |
| **Stop all** | Space | everything fades out and the stack is cleared |

Pushing a scene that's already in the stack moves it to the top.

### How a change is applied

Every operation works the same way:

1. Change the stack.
2. Compute the **target mix** from the stack: for every sound, whether it plays and at what
   volume, using the rules above.
3. Move the audio engine from the current mix to the target mix with fades. Sounds already
   playing never restart; they only change volume, pause or resume.

The fade time is taken from the scene being pushed or removed (`transitionMs`). For a switch it's
the new scene's.

This is the same smart crossfade as in [0008](decisions/0008-scenes.md), applied to the stack.

## UI

- **Scene list** (left): grouped (for example "Phandalin", "Session 3").
- **Stack bar** (top): the current stack as chips, `Travelling music › Forest › Combat ×`.
- **Now playing** (centre): every playing sound, with its effective volume and a live slider.
- **Effects pad** (right): buttons for the stack's effects.
- **Mixer bar** (bottom): master and per-category volumes.
- **Play / Edit mode:** in Play mode the library and scenes can't be changed by accident.
- **Save current mix to scene:** stores live tweaks back into a scene.

## Data (draft, in `comilito.json`)

```json
"scenes": [
  {
    "id": "scn-music",
    "name": "Travelling music",
    "hotkey": "1",
    "music": { "tracks": ["snd-travel-1", "snd-travel-2"], "shuffle": true },
    "ambience": [],
    "effects": [],
    "duck": 0.3,
    "transitionMs": 3000
  },
  {
    "id": "scn-forest",
    "name": "Forest",
    "group": "Wilderness",
    "hotkey": "2",
    "music": null,
    "ambience": [
      { "sound": "snd-birds",  "volume": 0.8, "role": "background" },
      { "sound": "snd-wind",   "volume": 0.5, "role": "background" },
      { "sound": "snd-stream", "volume": 0.4 }
    ],
    "effects": ["snd-twig"],
    "duck": 0.3,
    "transitionMs": 2500
  },
  {
    "id": "scn-combat",
    "name": "Combat",
    "hotkey": "F1",
    "music": { "tracks": ["snd-battle-drums"], "shuffle": false },
    "ambience": [
      { "sound": "snd-clash",   "volume": 0.8, "role": "foreground" },
      { "sound": "snd-goblins", "volume": 0.6, "role": "supporting" }
    ],
    "effects": ["snd-screech", "snd-sword", "snd-arrow"],
    "duck": 0.3,
    "transitionMs": 1500
  }
]
```

- `music`: `null` when the scene has no music, which leaves lower music playing.
- `duck`: how much this scene lowers the ambience below it (0.3 = by 30%; 0 = no ducking; 1 = silence).
- `role`: optional; one of `foreground`, `background`, `supporting`.

The current stack is **not** saved in the project. Each session starts with an empty stack.
