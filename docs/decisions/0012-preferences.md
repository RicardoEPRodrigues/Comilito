# 0012. Preferences window with app and project levels

- Status: Accepted
- Date: 2026-11-12

## Context

Default levels (ducking, fades, volumes, transition time) shouldn't be hard-coded; the user wants
a preferences window to set them. Projects are synced between computers (0006), so where these
defaults live affects whether a project sounds the same everywhere.

## Options considered

- **App preferences only, read live:** simple, but the same project sounds different on two
  computers with different preferences.
- **Every value copied into each scene/sound:** consistent, but changing "the default" later
  has no effect on existing scenes.
- **Two levels:** app preferences seed new projects; each project stores its own defaults, which
  scenes and sounds inherit unless they override them.

## Decision

Two levels. See [../preferences.md](../preferences.md).

- **App preferences** (`preferences.json` in the OS config folder): per-computer settings
  (output device, pause/stop fades, projects folder, behaviour, theme) and the defaults for new projects.
- **Project defaults** (`defaults` in `comilito.json`): ducking level, transition time, sound
  volume, and per-category loop/fade/shuffle. They're always written in full.
- Scene and sound fields like `duckLevel` or `fadeInMs` become optional and inherit from the
  project defaults when missing.
- App preferences never silently change an existing project.

## Consequences

- One `resolve` step in Rust (`value.or(project_default)`) before computing the mix. The mix
  calculation itself only ever sees concrete numbers.
- The editor shows inherited vs. overridden values and offers "reset to default".
- App preferences are the only data stored outside `~/Documents/Comilito` (consistent with 0005).
- Choosing an output device depends on kira's cpal backend, to be verified when the audio engine is built.
