# 0028. Each scene has its own music, ambience and effects volumes

- Status: Accepted
- Date: 2026-11-14

## Context

The Music, Ambience and Effects volumes were project-wide. Scenes want different balances:
a tavern wants the music low under the chatter, a battle wants it loud. The DM had to move the
sliders by hand every time the scene changed.

## Options considered

- **Per-scene multipliers on that scene's own sounds only:** each scene scales just its sounds.
  Confusing with the stack: the Ambience slider wouldn't affect ambience coming from scenes
  below, and the music often comes from a lower scene.
- **The top-most enabled scene sets the category volumes for everything playing:** one set of
  three volumes at a time, easy to explain ("the bar shows the top scene"), and an overlay
  scene can turn down the music it doesn't own.
- **Copy the project volumes into every scene** vs **inherit them while a scene has none:**
  copying makes old scenes stop following the project volumes; inheriting matches how
  `duckLevel` and `transitionMs` already work.

## Decision

- A scene may have `volumes` (`{ "music", "ambience", "effects" }`, linear 0–1). Without it, the
  project's category volumes (`categories.*.volume`) are used.
- The **top-most enabled scene** in the stack decides the three category track volumes, for
  every sound playing, including music from a scene below it. When it changes, the tracks fade
  to the new volumes over the same transition as the sounds.
- The **master volume stays project-wide.**
- The bottom bar's Music, Ambience and Effects sliders edit the top scene's volumes, and show its
  name. The first change gives a scene without volumes a copy of the project's, so only that one
  category changes. With no enabled scene, the sliders edit the project's volumes.
- Optional field: no format version change; older files load unchanged.

## Consequences

- Turning a scene into a quieter overlay is a matter of putting it on top and moving a slider.
- The project's category volumes become "the volumes of scenes that don't set their own". They're
  edited only when the stack has no enabled scene. Resetting a scene back to the project volumes
  isn't possible from the UI yet.
- The scene editor doesn't show the volumes; it keeps the scene's current ones when saving, so
  moving a slider while the editor is open isn't lost.
