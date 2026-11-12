# 0004. `.comilito` zip bundle with optional audio and relative paths

- Status: Superseded by [0006](0006-project-folders.md) (export options and relinking still apply)
- Date: 2026-11-12

## Context

Setups need to move between computers, and there should be a single save file with
everything needed for a session. Audio libraries can be large, so copying them shouldn't be required.

## Options considered

- **Config only (JSON with paths):** small, but breaks when paths differ between machines.
- **Bundle that always includes audio:** always works, but files can get very large.
- **Bundle with optional audio + relinking:** flexible, at the cost of a relinking flow on import.

## Decision

- One format: a `.comilito` file, which is a zip containing `comilito.json` and an optional `audio/` folder.
- On export the user chooses whether to include audio.
- Paths are stored relative to the project/bundle location.
- Each sound also stores `fileName`, `size` and `sha256` to help find files again.
- On import, missing files are listed. The user can pick a folder (searched recursively, many
  sounds fixed at once), locate files one by one, or skip.
- The format is versioned with `formatVersion`.

Details: [../save-format.md](../save-format.md).

## Consequences

- The Rust side needs zip read/write (`zip` crate) and hashing (`sha2` crate).
- A "Missing audio" dialog is part of the core UI, not an afterthought.
- Format changes need migrations for older `formatVersion`s.
