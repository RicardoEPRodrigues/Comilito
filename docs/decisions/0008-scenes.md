# 0008. Scenes: saved mixes with smart crossfades

- Status: Accepted
- Date: 2026-11-12

## Context

During play the mood changes quickly. For example, a calm wilderness trek turns into a goblin
ambush. The DM needs to change music and ambience together with one action, while some sounds
(for example, nature ambience) keep playing underneath.

## Options considered

- **Manual only:** the DM toggles each sound. This is too slow mid-session.
- **Scenes that stop everything and start a new set:** simple, but shared sounds restart audibly
  and the nature layer can't continue.
- **Scenes with smart crossfades:** sounds in both scenes keep playing and only change volume.
  The rest fade out or in.
- **Overlay scenes** (extension): a scene layered on top of the current one, which can be
  removed to restore it.

## Decision

Separate **sounds** (the library) from **scenes** (saved mixes). See [../scenes.md](../scenes.md).

- A scene holds: a music playlist, a set of ambience layers with volumes, and an effects pad.
- Switching scenes does a smart crossfade:
  - A sound in both scenes keeps playing and tweens to its new volume.
  - A sound only in the old scene fades out.
  - A sound only in the new scene fades in.
  - Music crossfades if the track changes.
- Live tweaks are allowed, and "Save current mix to scene" stores them.
- Play mode and Edit mode are separate.

Scenes can be layered; see [0010](0010-scene-stack.md) (which supersedes [0009](0009-overlay-scenes-and-roles.md)).

## Consequences

- The audio engine needs "play set X with volumes V" as an operation, not only play/stop.
- `comilito.json` gains a `scenes` list that refers to sounds by id.
- Deleting a sound must also remove it from every scene (the UI should warn first).
