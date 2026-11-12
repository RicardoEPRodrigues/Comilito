# 0006. A project is a folder; zip is only for export/import

- Status: Accepted
- Date: 2026-11-12
- Supersedes: [0004](0004-save-format.md) (the zip as the save file) and point 2 of [0005](0005-storage-location-and-hashing.md)

## Context

With the `.comilito` zip as the working save file, two problems showed up:

- Importing a bundle and extracting its audio left the same audio on disk twice, which wastes
  space in a synced folder.
- Any change to the project (even a volume tweak) means rewriting the whole zip, and the sync
  tool has to re-upload it.

## Options considered

- **A. Play straight from the zip** (audio stored uncompressed, streamed from byte ranges):
  no duplication, but every save rewrites the whole file.
- **B. Shared audio library + small save file:** no duplication, but the audio lives apart from
  the project.
- **C. Project folder:** a plain folder with `comilito.json` and an `audio/` folder.
- **SQLite as a single-file project:** updates in place and can stream audio, but cloud sync tools
  that re-upload whole files (Google Drive, OneDrive, Nextcloud) would re-upload everything on
  every change. It also risks corruption if synced while open, and can't be edited in a text editor.

## Decision

**C.** Each project is a self-contained folder under `~/Documents/Comilito/`:

```
~/Documents/Comilito/
├── Curse of Strahd/
│   ├── comilito.json
│   └── audio/{music,ambience,effects}/
└── Lost Mine of Phandelver/
    ├── comilito.json
    └── audio/...
```

- Each project has its own audio. Sounds added to a project are **copied** into its `audio/` folder.
  Projects don't share files, so the same sound in two projects is stored twice. This is accepted,
  because it keeps every project independent.
- Paths in `comilito.json` are relative to the project folder.
- The `.comilito` zip remains only as an **export/import format**: the same folder layout zipped,
  with or without audio. Import unpacks it into a new project folder. That keeps one layout and
  one schema; the zip is only a container for moving a project between computers.

## Consequences

- Syncing a project means syncing a folder. Only changed files are transferred, and audio rarely changes.
- `comilito.json` is written atomically (write a temporary file, then rename) so a sync tool
  never picks up a half-written file.
- Opening the same project on two devices at the same time can still cause sync conflicts on
  `comilito.json`. The app should detect external changes to the file and offer to reload.
- The project list is found by scanning `~/Documents/Comilito/` for folders that contain `comilito.json`.
- Relinking (pick a folder, locate a file, or skip) is still needed when importing a zip exported
  without audio. Relinked files are copied into the project, so it becomes self-contained again.
