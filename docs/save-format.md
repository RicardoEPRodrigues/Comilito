# Save format

> Status: implemented (format version 1). See decisions [0005](decisions/0005-storage-location-and-hashing.md) and
> [0006](decisions/0006-project-folders.md).

## Projects are folders

Each project is a self-contained folder under `~/Documents/Comilito/` (the OS Documents folder on Windows):

```
~/Documents/Comilito/
└── Curse of Strahd/
    ├── comilito.json
    └── audio/
        ├── music/
        ├── ambience/
        └── effects/
```

- Adding a sound **copies** the file into the project's `audio/<category>/` folder.
- All paths in `comilito.json` are relative to the project folder.
- Comilito lists projects by scanning `~/Documents/Comilito/` for folders containing `comilito.json`.
- `comilito.json` is written atomically (temporary file, then rename) so that sync tools never
  see a half-written file.

To sync between devices, put the project folders in a synced location. Only changed files are
transferred. Avoid having the same project open on two devices at the same time.

## `comilito.json` (draft)

```json
{
  "formatVersion": 1,
  "name": "Curse of Strahd",
  "masterVolume": 1.0,
  "categories": {
    "music":    { "volume": 0.8 },
    "ambience": { "volume": 0.6 },
    "effects":  { "volume": 1.0 }
  },
  "sounds": [
    {
      "id": "b3f1c2e0-...",
      "category": "music",
      "name": "Castle Ravenloft",
      "file": {
        "path": "audio/music/castle-ravenloft.ogg",
        "size": 4821330,
        "modified": "2026-11-12T15:04:00Z",
        "sha256": "9f2c...",
        "loudness": { "lufs": -21.4, "peak": 0.89 }
      },
      "volume": 1.0,
      "loop": true,
      "fadeInMs": 2000,
      "fadeOutMs": 2000
    }
  ],
  "scenes": [],
  "stack": [],
  "stackPresets": [],
  "defaults": { "...": "see preferences.md" },
  "backups": { "intervalMinutes": 30, "keep": 1, "includeAudio": false }
}
```

- `scenes`, `stack`, `stackPresets`: see [scenes.md](scenes.md).
- `defaults`: project-wide default levels; see [preferences.md](preferences.md). Per-sound `volume`, `loop`, `fadeInMs` and `fadeOutMs` are optional and inherit from it.

- `formatVersion`: bumped on breaking changes. Older versions must still load (migrated on load).
  Adding an optional field (like a scene's `blocksMusic`) isn't a breaking change: older files
  simply don't have it, and older versions of the app ignore it.
- `path`: relative to the project folder.
- `size`, `modified`, `sha256`: used to find missing files. `sha256` may be absent until hashing finishes.
- `loudness`: measured in the background for loudness normalization
  ([0015](decisions/0015-loudness-normalization.md)); absent until measured.

## Export and import: the `.comilito` zip

To move a project to a computer without a synced folder, export it as a `.comilito` file. This is
a zip of the project folder with the same layout. Export offers one choice:

- **Include audio:** the whole folder is zipped. Audio is stored without compression, since
  mp3/ogg/flac are already compressed.
- **Without audio:** only `comilito.json` is zipped.

Import unpacks the zip into a new folder under `~/Documents/Comilito/` (renamed if that name
already exists). The zip can then be deleted, so nothing is stored twice. Only `comilito.json`
and files under `audio/` are extracted, and paths that would escape the new folder are refused.

### Missing files

After import (or whenever a project is opened), any sound whose file is missing is listed in a
**"Missing audio" dialog**. The user can:

- **Pick a folder:** searched recursively for files with the same name. If several match, `size`
  and then `sha256` break the tie. One folder can fix many sounds at once.
- **Locate a single file:** a file picker for that one sound.
- **Skip:** the sound stays marked as missing and can be fixed later.

Relinked files are **copied** into the project's `audio/` folder, so the project becomes
self-contained again.

## Backups

See [decision 0007](decisions/0007-backups.md). Backups are `.comilito` zips written to
`~/Documents/Comilito/Backups/<Project name>/`:

- every 30 minutes while there are changes, and when the project is closed
- the newest 1 is kept (configurable); the new backup is written before the oldest is deleted
- audio is not included by default (configurable)

Removing a sound asks whether to move its audio to the system trash (default) or delete it permanently.

## Hashing

`sha256` is computed lazily on a background thread:

- The app stays usable, and a status indicator shows progress.
- Results are stored with the file's size and modification time, and recomputed only if the file changes.
- Unfinished work simply resumes on the next launch (sounds without `sha256` are picked up again).
  The quit warning from decision 0005 isn't implemented yet: quitting mid-hash loses nothing.
- Export doesn't wait for hashes: relinking matches by file name and size first, and only needs
  the hash to choose between several same-sized files with the same name.
