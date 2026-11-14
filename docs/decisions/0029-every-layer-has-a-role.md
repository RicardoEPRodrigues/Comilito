# 0029. Every ambience layer has a role (Foreground by default)

- Status: Accepted
- Date: 2026-11-14

## Context

Ambience layers could have a role (`foreground`, `background`, `supporting`) or none
([0009](0009-overlay-scenes-and-roles.md)). A layer without a role was ducked by every enabled
scene above it, while a layer with a role was ducked only by scenes that have a layer in the same
role. New layers started with no role, so the "No role" option was what most layers had, and it
behaved differently from the three named roles in a way that was hard to see. The role could
only be changed in the scene editor.

## Options considered

- **Keep "No role":** no change for existing projects, but two ducking rules to explain.
- **Remove it; layers without a role become Foreground:** one rule for every layer. Scenes above
  usually have foreground layers too, so most mixes sound the same. A scene with no ambience at
  all (music only) no longer ducks anything below it.
- **Remove it; layers without a role become Background:** background layers are meant to run
  under everything, so turning every unlabelled layer into one would change far more mixes.

## Decision

- Every layer has a role. New layers (added in the scene editor, or by dropping a sound on a
  scene) start as **Foreground**. The "No role" option is gone.
- A layer is ducked by an enabled scene above only if that scene has a layer with the same role.
- In `comilito.json`, `role` is always written. A layer without one (older files) loads as
  `foreground`. The format version stays 2: older versions of the app read the field as before.
- The role can also be changed from the tapestry: each layer in an expanded stack entry has a
  role picker (`set_layer_role` command). The change fades in like a scene edit.

## Consequences

- One ducking rule, and every layer shows its role.
- Older projects can sound different in one case: a scene without ambience (for example music
  only) placed above other scenes used to duck their unlabelled layers and now leaves them alone.
  The fix is to give that scene a foreground layer, or to lower its ambience volume (0028).
