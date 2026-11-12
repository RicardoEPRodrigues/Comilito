# 0009. Overlay scenes with layer roles

- Status: Superseded by [0010](0010-scene-stack.md) (roles are kept)
- Date: 2026-11-12
- Extends: [0008](0008-scenes.md)

## Context

Some moods are temporary and happen in many places. Combat is the clearest example: the same
"Combat" mood should work in a forest, a cave or a city, and when it ends the previous scene
should come back exactly as it was. Some of the underlying ambience (for example, nature) should
keep playing unchanged during the overlay.

## Options considered

- **Only regular scenes:** works, but needs a "goblin ambush" copy of every location scene.
- **Overlay that ducks everything underneath:** simple, but can't keep chosen layers at full volume.
- **Overlay with layer roles:** layers have a role, and the overlay only ducks the roles it takes over.

## Decision

Overlay scenes with layer roles.

**Roles.** Each ambience layer *in a scene* may have a role: `foreground`, `background` or
`supporting`. The role belongs to the scene layer, not the sound, so the same sound can have
different roles in different scenes.

**While an overlay is active**, each layer of the underlying scene is treated as follows:

| Underlying layer | Effect |
|---|---|
| has no role | ducked to the overlay's `duckLevel` (default 30%) |
| has role R, and the overlay has a layer with role R | ducked to `duckLevel` (the overlay takes over that role) |
| has role R, and the overlay has no layer with role R | plays unchanged |

A `duckLevel` of 0 silences ducked layers.

**Music.** If the overlay has music, the underlying music fades out and **pauses**, and the
overlay music fades in. When the overlay ends, the underlying music resumes from where it paused.
If the overlay has no music, the underlying music continues.

**Effects pad.** The overlay's effects come first, followed by the scene's.

**Lifecycle.**

- Only one overlay is active at a time (v1). Triggering another overlay replaces the current one.
- Switching the base scene while an overlay is active crossfades the base underneath, and the
  overlay stays.
- Ending the overlay fades out its layers, restores ducked layers to their scene volumes and
  resumes the underlying music.

A scene has a `kind` of `scene` or `overlay`. Both are edited the same way; overlays also have `duckLevel`.

## Consequences

- The audio engine tracks two layers of state, base and overlay. The volume of each playing sound is
  computed from its scene volume, the duck factor, the category volume and the master volume.
- Music needs pause/resume with fades as well as play/stop (kira supports this).
- Stacked overlays (for example, Combat plus Storm) are possible later but out of scope for v1.
