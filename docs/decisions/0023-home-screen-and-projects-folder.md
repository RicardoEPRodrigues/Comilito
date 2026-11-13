# 0023. Home screen, recent projects and moving the projects folder

- Status: Accepted
- Date: 2026-11-13

## Context

The owner asked for a redesigned home screen (the project picker), drawn in Pencil first:
- an option to open the previous project at start;
- duplicating projects;
- Import and Restore tucked under a menu.

They also asked for a setting to migrate the projects folder.

## Options considered

- **Remembering the last project:** a single "last project" path, or a short recent list. The
  list costs little more, and it also orders the home screen by recent use.
- **Who writes the recent list:** the UI, or Rust when a project opens. If the UI wrote it, a
  Preferences window opened earlier would save an old copy over a newer one.
- **Moving the projects folder:**
  - move the folder itself;
  - copy everything and leave the old folder;
  - move only Comilito's folders.

  A user may have pointed the projects folder at a shared folder such as `~/Documents`, so
  moving the whole folder could carry unrelated files along.

## Decision

- **Home screen** (design "H · Home — projects" in `pencil/comilito.pen`):
  - The brand column holds New project and a More menu (Import a project, Restore a backup,
    Open the projects folder), with the "Open the last project at start" switch at the bottom.
  - The last opened project gets a large Continue card.
  - Other projects follow, most recently opened first, with scene/preset/sound counts and when
    they were last edited.
  - Each project has a ⋯ menu (also right-click): Open, Duplicate, Show in file manager, Delete.
- **Recent projects** live in `preferences.json` (`recentProjects`, at most 20, most recent
  first) and are written only by Rust when a project opens. `set_preferences` keeps Rust's
  copy. `openLastProject` opens the first entry at start, or shows the home screen with the
  reason if that fails.
- **Duplicate** copies the folder to "<name> (copy)" next to it and renames the copy inside its
  `comilito.json`. An open project is saved first, so the copy matches what's on screen.
- **Move to…** (Preferences › Projects & tools) moves only folders containing a
  `comilito.json`, plus `Backups`.
  - Before moving anything, it checks that no name clashes and that the target isn't inside a
    folder being moved.
  - It renames when it can, and copies then deletes across disks.
  - An open project is closed and reopened from its new place.
  - The old folder is removed only if it ends up empty.
  - The recent list is updated to the new paths.
- **Choose…** stays as it was: it only changes where Comilito looks, for a folder synced from
  another computer, for example.

## Consequences

- `preferences.json` gains `openLastProject` and `recentProjects`, both optional with defaults,
  so older files still load.
- A move across disks copies everything while holding the app state lock, so the app is
  unresponsive until it finishes. That's acceptable for an occasional action; a progress
  dialog would need the copy moved to a background thread.
