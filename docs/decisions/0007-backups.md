# 0007. Automatic backups using the export zip

- Status: Accepted
- Date: 2026-11-12

## Context

Projects are plain folders (0006) that are edited live and may be synced. A bad edit, a sync
conflict or an accidental deletion should be recoverable.

## Decision

Backups reuse the export format: a backup is a `.comilito` zip of the project.

- **Location:** `~/Documents/Comilito/Backups/<Project name>/`. This is outside the project
  folder, so backups are never included in exports and don't clutter the project.
- **File name:** `<Project name> YYYY-MM-DD HH-MM.comilito`.
- **When:** every *N* minutes while the project is open (default 30), and when the project is closed.
  Nothing is backed up if nothing changed since the last backup.
- **Retention:** keep the newest *K* backups (default **1**, configurable). The new backup is
  written and checked first, and only then is the oldest deleted. There is never a moment with
  zero backups.
- **Audio:** not included by default (configurable per project). Audio rarely changes, and including
  it makes every backup a full copy of the project.
- **Restoring** a backup works like importing it, but the user chooses whether to replace the
  current project or restore it as a copy.

## Consequences

- Removing a sound asks the user: **Move to trash** (default) or **Delete permanently**. Trash is the
  default because a config-only backup can't bring the audio back. If any scenes use the sound,
  the prompt lists them. This uses the `trash` crate (small, maintained, works on Linux, Windows and macOS).
- Backups run on the same background worker as hashing, so the UI never freezes.
- Backup settings (interval, count, include audio) are stored per project in `comilito.json`.
