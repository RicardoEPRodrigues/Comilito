# Save format

> Status: draft. See [decision 0004](decisions/0004-save-format.md).

## The `.comilito` bundle

A `.comilito` file is a **zip archive**. Rename it to `.zip` and you can open it with any archive tool.

```
my-campaign.comilito
├── comilito.json        # the project config (always present)
└── audio/               # only when exported "with audio"
    ├── music/
    ├── ambience/
    └── effects/
```

Export offers one choice: **include audio files** (on/off).

- **On:** every referenced file is copied into `audio/`, and paths in `comilito.json` point there.
  The bundle is self-contained.
- **Off:** only `comilito.json` is stored. Paths stay relative to where the original project
  lived, and each sound keeps its file name, size and hash so it can be found again.

## `comilito.json` (draft)

```json
{
  "formatVersion": 1,
  "name": "Curse of Strahd",
  "masterVolume": 0.9,
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
        "fileName": "castle-ravenloft.ogg",
        "size": 4821330,
        "sha256": "9f2c..."
      },
      "volume": 1.0,
      "loop": true,
      "fadeInMs": 2000,
      "fadeOutMs": 2000
    }
  ]
}
```

- `formatVersion` — bumped on breaking changes. Older versions must still load (migrated on load).
- `path` — **relative** to the folder holding the config (or to the bundle root). Absolute paths
  are only a fallback for files on a different drive.
- `fileName`, `size`, `sha256` — used to find missing files again.

## Importing and missing files

On import, Comilito resolves each sound's file in this order:

1. Inside the bundle (`audio/…`), if present.
2. The relative `path`, starting from the bundle's folder.
3. The absolute path, if one was stored.

Any file still missing goes into a **"Missing audio" dialog**, where the user can:

- **Pick a folder:** Comilito searches it recursively for files matching `fileName`. If several
  match, `size` and then `sha256` break the tie. One folder can fix many sounds at once.
- **Locate a single file:** a file picker for that one sound.
- **Skip:** the sound stays in the project, marked as missing, and can be fixed later.

Imported bundles with audio are extracted into the app's data folder (or a folder the user
chooses), so playback never reads straight from the zip.

## Open questions

- Where extracted bundle audio lives by default (app data folder vs. next to the bundle).
- Whether to also allow saving as a plain `.comilito.json` outside a zip (handy for git or hand-editing).
- Hashing large libraries is slow: hash lazily or only on export?
