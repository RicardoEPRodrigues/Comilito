# Save format

> Status: implemented (format version 1). See decisions [0005](decisions/0005-storage-location-and-hashing.md) and
> [0006](decisions/0006-project-folders.md).

## Projects are folders

Each project is a self-contained folder under `~/Documents/Comilito/` (the OS Documents folder on Windows):

```
~/Documents/Comilito/
└── Curse of Strahd/
    ├── comilito.json
    ├── audio/
    │   ├── music/
    │   ├── ambience/
    │   └── effects/
    └── video/        (only once videos are added, decisions 0035 and 0039)
        ├── ambience/
        └── effects/
```

- Adding a sound **copies** the file into the project's `audio/<category>/` folder.
- Audio files put into `audio/<category>/` (or its subfolders) outside the app are picked up
  automatically as new sounds while the project is open. Deleting a file there marks its sound
  as missing; the missing-audio banner offers to relink it or remove such sounds.
- All paths in `comilito.json` are relative to the project folder.
- Comilito lists projects by scanning `~/Documents/Comilito/` for folders containing `comilito.json`.
- `comilito.json` is written atomically (temporary file, then rename) so that sync tools never
  see a half-written file.

To sync between devices, put the project folders in a synced location. Only changed files are
transferred. Avoid having the same project open on two devices at the same time.

## `comilito.json` (draft)

```json
{
  "formatVersion": 2,
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
- `defaults`: project-wide default levels; see [preferences.md](preferences.md). Per-sound `volume`, `loop`, `fadeInMs`, `fadeOutMs` and `normalize` are optional and inherit from it (`normalize` from `defaults.normalize`, the switch for the sound's category).

- `formatVersion`: bumped on breaking changes. Older versions must still load (migrated on load).
  Adding an optional field (like a scene's `blocksMusic`) isn't a breaking change: older files
  simply don't have it, and older versions of the app ignore it.
- `path`: relative to the project folder.
- `size`, `modified`, `sha256`: used to find missing files. `sha256` may be absent until hashing finishes.
- `loudness`: measured in the background for loudness normalization
  ([0015](decisions/0015-loudness-normalization.md)); absent until measured.

### Format version 2

Version 2 (2026-11-13) only adds optional fields, so version 1 files load unchanged and are
written back as version 2. On each sound:

- `tags`: labels for searching, stored without `#`, lower-case (`["forest", "night"]`).
- `favourite`: `true` for starred sounds (omitted when false).
- `source`: for downloaded audio, `{ url, title, site, downloadedAt, range? }`, where `range` is
  `{ startMs, endMs }` when only part was downloaded (decision 0019).
- `lastUsed`: when the sound was last previewed, played or added to a scene, for "Recent".

On each scene (added 2026-11-14, still version 2):

- `volumes`: `{ music, ambience, effects }`, the category volumes while the scene is on top of the
  stack ([0028](decisions/0028-scene-volumes.md)). Without it, the project's `categories` volumes
  are used, so `categories` now means "the volumes of scenes that don't set their own".
- `ducksAll`: optional, only written when `true`: the scene ducks all the ambience below it,
  whatever the role ([0031](decisions/0031-scenes-that-duck-all-ambience.md)).
- Ambience layers: `role` is always written; a layer without one loads as `foreground`
  ([0029](decisions/0029-every-layer-has-a-role.md)). Older versions of the app read it as before.
- `file.loudness.durationMs`: the length, measured with the loudness. Files measured before
  this existed are measured again in the background.

On the project:

- `presetGroups`: preset groups in display order, including empty ones; each preset may have a
  `group` (decision 0017).
- `stackOrigin`: the preset the current stack was loaded from or last saved to, for
  "From … · Modified", Update and Revert.

Library folders aren't stored: they're the subfolders of `audio/<category>/` on disk. Moving a
folder (dragging it onto another folder or its category in the Library, or *Move to* in its
menu) moves it on disk within its category and rewrites the `file.path` of every sound inside.

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

See decisions [0007](decisions/0007-backups.md) and [0026](decisions/0026-backups-in-app-data.md).
Backups are `.comilito` zips written to `<app data folder>/backups/<Project name>/`
(`~/.local/share/com.comilito.app/backups/` on Linux, `%APPDATA%\com.comilito.app\backups\` on
Windows). Versions before 0026 wrote them to `<projects folder>/Backups/`; they're moved over the
next time Comilito starts.

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

### Videos (added 2026-11-15, still version 2)

Videos ([0035](decisions/0035-video.md)) only add optional fields, so older files load
unchanged, and a project without videos is written exactly as before (apart from
`defaults.video`, since defaults are always written in full).

- `videos`: written only when there are some. Each is like a sound without the audio settings:

  ```json
  {
    "id": "vid-…",
    "name": "Castle gate at dusk",
    "kind": "ambience",
    "file": { "path": "video/ambience/ravenloft/castle-gate.webm", "size": 412000000, "modified": "…" },
    "info": { "container": "matroska,webm", "codec": "vp9", "width": 1920, "height": 1080,
              "durationMs": 760000, "playsEverywhere": true },
    "tags": ["castle"], "favourite": true, "source": { "...": "as for sounds" }, "lastUsed": "…"
  }
  ```

  `kind` is `"ambience"` (for scenes' playlists) or `"effect"` (for video effects); missing means
  ambience ([0039](decisions/0039-ambience-videos-and-video-effects.md)). The file is under
  `video/ambience/` or `video/effects/` to match. Projects from before kinds kept videos straight
  in `video/`; they're moved into `video/ambience/` (and the paths updated) the first time the
  project opens.
  `info` is measured in the background with ffprobe and is absent until then.
  `playsEverywhere` is true for WebM with VP8, VP9 or AV1; other videos are offered for
  conversion.
- On a scene, all optional:
  - `video`: `{ "videos": ["vid-…"], "shuffle": false, "switchMinutes": 10, "resume": true }`.
    The three settings are overrides of `defaults.video`; `switchMinutes: 0` plays each video
    to its end. Without `video`, the scene leaves the video of the scenes below showing.
  - `hidesVideo`: `true` to hide the video of the scenes below (only meaningful without `video`).
  - `videoEffects`: video ids for the video effects pad.
- `defaults.video`: `{ "shuffle": false, "switchMinutes": 10, "resume": true }`.
- `backups.includeVideo`: `false` by default; videos are big.

