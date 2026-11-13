# 0026. Keep backups in the app's data folder, not the projects folder

- Status: Accepted (changes where 0007 stores backups)
- Date: 2026-11-13

## Context

Decision 0007 wrote backups to `<projects folder>/Backups/`, next to the projects. The owner
wants the projects folder (`~/Documents/Comilito` by default) to hold only projects, with
backups kept with the app's own files.

## Options considered

- **Keep `<projects folder>/Backups`:** backups travel with the projects when the folder is
  synced or moved, but they clutter a folder people browse, and they double what a sync tool
  uploads.
- **The config folder (`~/.config/comilito`):** where preferences are, but config folders are
  meant for small settings files, and backups can include audio (hundreds of MB).
- **The app data folder (`~/.local/share/com.comilito.app/backups`, `%APPDATA%\com.comilito.app\backups`):**
  meant for app-managed data, already holds logs and the downloaded tools.

## Decision

- Backups go to `<app data folder>/backups/<Project name>/`.
- At start, a background thread moves any `<projects folder>/Backups/` there. A backup already
  in the new place with the same name is never overwritten; the old one is then left behind.
- Moving the projects folder (0023, 0024) moves only projects now.
- "Remove all data and quit" asks whether to keep the backups or delete them too, since they now
  live in the folder it deletes.

## Consequences

- The projects folder is just projects. Syncing it no longer syncs backups; to copy backups to
  another computer, use "Restore a backup…" on files copied from the data folder.
- Backups are per computer, like preferences.
- Restore ("Restore a backup…") opens the new folder, so nothing changes for the user there.
