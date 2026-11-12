# Scenes

> Status: accepted ([0008](decisions/0008-scenes.md), [0010](decisions/0010-scene-stack.md),
> [0011](decisions/0011-stack-ui-and-presets.md)).

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

What plays is decided by a **stack of scenes**. Any scene can go anywhere in the stack, there's
no limit on its depth, and each entry can be **enabled or disabled** without removing it.
Disabled entries keep their position but are ignored when working out the mix.

```
  ┌──────────────────────┐
  │ ☑ Combat             │  ← top: drums, battle clash, goblin chatter
  ├──────────────────────┤
  │ ☐ Rain   (disabled)  │  ← kept in place, not playing
  ├──────────────────────┤
  │ ☑ Forest             │  ← birds, wind, stream (no music)
  ├──────────────────────┤
  │ ☑ Travelling music   │  ← bottom: music only
  └──────────────────────┘
```

The stack is **saved in the project**, so each session starts from where the last one ended.
On opening a project the stack is loaded but **paused**; press Space to start.

### Music

Only the **top-most enabled scene that has music** plays its music. Music from lower scenes fades
out and **pauses**, and resumes where it left off when it's uncovered. This includes when the scene
covering it is disabled. Music is never ducked.

### Ambience and ducking

Every enabled scene **ducks** the ambience of the enabled scenes below it to its `duckLevel`
(default **30%**: a layer plays at 30% of its volume). Ducking multiplies, once per enabled
scene above that ducks the layer:

| Stream (40%) with… | Effective volume |
|---|---|
| nothing above | 40% |
| 1 scene above | 40% × 30% = 12% |
| 2 scenes above | 40% × 30% × 30% = 3.6% |

Each scene uses its own `duckLevel`, so a scene with `duckLevel` 0.5 above one with 0.3 gives 40% × 0.3 × 0.5 = 6%.

**Roles** let a layer escape ducking. Each ambience layer can have a role: `foreground`,
`background` or `supporting`. For each enabled scene above a layer:

| Layer below | Scene above… | Effect |
|---|---|---|
| no role | (any) | ducked |
| role R | has a layer with role R | ducked (that role is taken over) |
| role R | has no layer with role R | not ducked by this scene |

### Effects pad

Shows the effects of every enabled scene in the stack, top scene first.

### Same sound in several scenes

A sound plays only once. If several enabled stack entries contain it, the **highest** entry's
volume (after its own ducking) is used. This also covers the same scene added twice.

### Example

| Sound | Stack: Music + Forest | + Combat on top |
|---|---|---|
| 🎵 Travelling theme (Music) | ▶ playing | ⏸ paused |
| 🎵 Battle drums (Combat) | — | ▶ fades in |
| 🌲 Birds (Forest, background, 80%) | 80% | 80%: Combat has no background layer |
| 🌲 Wind (Forest, background, 50%) | 50% | 50% |
| 🌲 Stream (Forest, no role, 40%) | 40% | 12% (ducked to 30%) |
| ⚔️ Battle clash (Combat, foreground, 80%) | — | 80% |
| ⚔️ Goblin chatter (Combat, supporting, 60%) | — | 60% |

Disabling or removing Combat restores the left column.

## Stack presets

A **preset** is a saved stack (its entries, order and enabled state) with a name, for example
"Forest ambush" or "Tavern night".

- **Save stack as preset** stores the current stack.
- **Load preset** (right-click → *Replace stack*) replaces the whole stack.
- **Dragging a preset** into the stack inserts its scenes at the drop position.

## Controls

The stack is the focus, and it's driven mainly by drag and drop.

| Action | How |
|---|---|
| Add a scene | drag it from the list into the stack, at any position (top, bottom, between two scenes) |
| Reorder | drag an entry within the stack |
| Enable / disable | the checkbox on the entry |
| Remove | × on the entry, or drag it out of the stack |
| Add on top | right-click a scene → *Add on top* |
| Switch stack | right-click a scene → *Switch stack to this scene* (the stack becomes just this scene) |
| Replace with preset | right-click a preset → *Replace stack* |
| Scene hotkey | not in the stack → added on top; in the stack → toggles enabled/disabled |
| Pause / resume | **Space** pauses everything (positions kept); Space again resumes |
| Stop all | **Esc** fades everything out and resets positions; the stack is kept |

**Duplicates:** adding a scene that's already in the stack asks:
*"Forest is already in the stack. Move it here / Add a duplicate / Cancel."*

### How a change is applied

Every operation works the same way:

1. Change the stack.
2. Compute the **target mix** from the enabled entries: for every sound, whether it plays and
   at what volume, using the rules above.
3. Move the audio engine from the current mix to the target mix with fades. Sounds already
   playing never restart; they only change volume, pause or resume.

The fade time is the `transitionMs` of the scene being added, removed or toggled. For a switch
or preset load it's the top scene's.

## UI layout (draft)

```
┌ Library ─────────────────┬ Stack ────────────────────────────┬ Effects ───────┐
│ PRESETS                  │ ▲ top                             │ [Goblin screech]│
│  ▸ Forest ambush         │ ┌───────────────────────────────┐ │ [Sword hit]     │
│  ▸ Tavern night          │ │ ⠿ ☑ Combat     🎵▶ ⚔️×2     ✕ │ │ [Arrow]         │
│                          │ ├───────────────────────────────┤ │ [Twig snap]     │
│ SCENES                   │ │ ⠿ ☐ Rain       disabled      ✕ │ │                 │
│ Wilderness               │ ├───────────────────────────────┤ │                 │
│  ▸ Forest      ── drag ──▶ │ ⠿ ☑ Forest     🌲×3          ✕ │ │                 │
│  ▸ Rain                  │ ├───────────────────────────────┤ │                 │
│ Battle                   │ │ ⠿ ☑ Travelling 🎵⏸           ✕ │ │                 │
│  ▸ Combat                │ └───────────────────────────────┘ │                 │
│                          │ ▼ bottom                          │                 │
│ [+ New scene]            │ [Save stack as preset]            │                 │
└──────────────────────────┴───────────────────────────────────┴─────────────────┘
 ⏯ Space   Master ▮▮▮▮▮▮▯   Music ▮▮▮▮▯   Ambience ▮▮▮▯   Effects ▮▮▮▮▮
```

- **Library** (left): presets and scenes, grouped (for example "Wilderness", "Session 3").
- **Stack** (centre): top of the stack at the top. `⠿` is the drag handle. Clicking an entry
  expands it to show its sounds with their *effective* volume and a live slider for each.
- **Effects pad** (right): buttons for the enabled scenes' effects.
- **Mixer bar** (bottom): play/pause, master and per-category volumes.
- **Play / Edit mode:** in Play mode the library and scenes can't be changed by accident. The
  stack can always be changed.
- **Save current mix to scene:** stores live slider tweaks back into a scene.

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
    "duckLevel": 0.3,
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
    "duckLevel": 0.3,
    "transitionMs": 2500
  },
  {
    "id": "scn-combat",
    "name": "Combat",
    "group": "Battle",
    "hotkey": "F1",
    "music": { "tracks": ["snd-battle-drums"], "shuffle": false },
    "ambience": [
      { "sound": "snd-clash",   "volume": 0.8, "role": "foreground" },
      { "sound": "snd-goblins", "volume": 0.6, "role": "supporting" }
    ],
    "effects": ["snd-screech", "snd-sword", "snd-arrow"],
    "duckLevel": 0.3,
    "transitionMs": 1500
  }
],
"stack": [
  { "id": "stk-1", "scene": "scn-music",  "enabled": true },
  { "id": "stk-2", "scene": "scn-forest", "enabled": true },
  { "id": "stk-3", "scene": "scn-combat", "enabled": true }
],
"stackPresets": [
  {
    "id": "pre-1",
    "name": "Forest ambush",
    "stack": [
      { "scene": "scn-music",  "enabled": true },
      { "scene": "scn-forest", "enabled": true },
      { "scene": "scn-combat", "enabled": true }
    ]
  }
]
```

- `music`: `null` when the scene has no music, which leaves lower music playing.
- `duckLevel`: the volume this scene leaves the ambience below it at (0.3 = 30%; 1 = no ducking; 0 = silence).
  Optional: if missing, the project default is used (see [preferences.md](preferences.md)).
- `role`: optional; one of `foreground`, `background`, `supporting`.
- `stack`: ordered **bottom to top**. Each entry has its own `id`, so a scene can appear twice.
  It's saved automatically (debounced) whenever it changes.
