# 0058. File backups by a project id, not only its name

- Status: Accepted (changes the folder name of [0026](0026-backups-in-app-data.md))
- Date: 2026-11-20

## Context

Backups go to `<app data folder>/backups/<Project name>/` (decision 0026), and after each backup
the oldest ones in that folder beyond the project's "keep" count are deleted. Two projects with
the same name, more likely since projects can live outside the projects folder (decision 0042),
or a duplicate renamed back, shared that folder: backing up one could delete the other's
backups.

## Options considered

- **Key the folder by the project folder's path:** no format change, but moving or renaming the
  project folder (or the projects folder, decisions 0023 and 0024) would orphan its backups.
- **An id in `comilito.json`:** travels with the project wherever it moves. An optional field, so
  not a breaking format change (format version stays 2); older versions of the app ignore it.

## Decision

- `comilito.json` gets `id`: `prj-` and a random UUID. New projects get one when created. A
  project without one gets one the first time it's opened, saved straight away so the next
  backup goes to the same folder. A duplicated project and an imported `.comilito` (a restored
  backup is a copy, maybe next to its original) get a new one.
- Backups go to `backups/<Project name> (<first 8 characters of the id>)/`: still readable in the
  "Restore a backup…" picker, and distinct per project.
- Folders from before (`backups/<Project name>/`) are left as they are: their backups can't be
  told apart by project, so they're neither moved nor pruned, and still show in the picker.

## Consequences

- Two same-named projects never touch each other's backups.
- A project folder copied by hand outside the app keeps the original's id and so shares its
  backups, as before; duplicating it in Comilito doesn't.
- Old backup folders stay until the user deletes them; they no longer grow.
