# 0010. Scene stack instead of a single overlay

- Status: Accepted
- Date: 2026-11-12
- Supersedes: [0009](0009-overlay-scenes-and-roles.md)

## Context

0009 allowed one base scene plus one overlay, with separate `scene` and `overlay` kinds. In
practice moods are built in layers. For example: travelling music that plays in any environment,
then a forest that adds ambience but no music, then combat that adds new music and battle sounds.
Each layer should be addable and removable on its own.

## Decision

Replace base + overlay with a **stack of scenes** of any depth. Any scene can be at any position;
there are no separate scene kinds.

- **Music:** only the top-most scene with music plays it. Lower music fades out and pauses,
  and resumes when it's uncovered. Music is never ducked.
- **Ambience:** each scene ducks the ambience of the scenes below it by its `duck` amount
  (default 30%, so 70% volume). Ducking multiplies across levels.
- **Roles** (from 0009, kept): a layer with role R is ducked only by scenes above it that also
  have a layer with role R. A layer with no role is ducked by every scene above it.
- **Same sound in several scenes:** it plays once, at the highest scene's volume.
- **Effects pad:** effects from all scenes in the stack, top first.
- **Operations:** switch (replace the stack), push, remove any scene, pop, stop all. Pushing a
  scene already in the stack moves it to the top.
- Every operation recomputes a **target mix** from the stack, and the engine fades from the
  current mix to it. Playing sounds never restart.
- The stack is session state and isn't saved in the project.

Details: [../scenes.md](../scenes.md).

## Consequences

- The scene `kind` and `duckLevel` fields from 0009 are replaced by a single `duck` field on every scene.
- The "target mix" calculation is a pure function (stack + scenes → volumes). It's easy to unit
  test in Rust without playing any audio, so it should be written and tested first.
- The audio engine only needs one high-level operation, "fade to this mix", on top of play,
  pause, resume, stop and set volume.
