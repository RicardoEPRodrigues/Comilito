# Preferences

> Status: accepted ([decision 0012](decisions/0012-preferences.md)).

Settings live at two levels:

| Level | Stored in | Applies to | Opened from |
|---|---|---|---|
| **App preferences** | `preferences.json` in the OS config folder (`~/.config/comilito/` on Linux, `%APPDATA%\comilito\` on Windows) | this computer: app behaviour, plus the starting defaults for **new** projects | *Preferences* window (Ctrl+,) |
| **Project defaults** | `comilito.json` (`defaults`) | every scene and sound in this project that doesn't set its own value | *Project settings* window |

### How a value is resolved

```
sound / scene value  →  if not set: project default  →  (always set; copied from app preferences when the project is created)
```

- Scenes and sounds store a value **only when it differs** from the project default. Changing
  a project default updates everything that inherits it. In the editor, inherited values are
  shown greyed out, with a "reset to default" button on overridden ones.
- Project defaults are always written in full to `comilito.json`. A project therefore sounds the
  same on every computer, whatever that computer's app preferences are.
- App preferences never change an existing project. They only seed new ones. A button in
  *Project settings* ("Reset to my app defaults") copies them over explicitly.

## Preferences window

### Levels (defaults for new projects)

| Setting | Default |
|---|---|
| Ducking level | 30% |
| Master volume | 100% |
| Music / Ambience / Effects category volume (for scenes without their own) | 80% / 60% / 100% |
| New sound volume | 100% |
| Scene transition time | 2.5 s |
| Loudness normalization | on for music and ambience, off for effects; target −18 LUFS ([0015](decisions/0015-loudness-normalization.md)) |

### Sound defaults per category (defaults for new projects)

| Setting | Music | Ambience | Effects |
|---|---|---|---|
| Loop | on | on | off |
| Fade in | 2 s | 1.5 s | 0 s |
| Fade out | 2 s | 1.5 s | 0 s |
| Shuffle playlist | off | — | — |

### Playback (this computer)

| Setting | Default |
|---|---|
| Audio output device | system default (follows the desktop). Or one of the outputs the desktop lists; if it isn't connected, the system default is used ([0032](decisions/0032-choosing-the-audio-output.md)) |
| Pause / resume fade (Space) | 0.3 s (a short fade avoids clicks) |
| Stop all fade (Esc) | 1 s |

### Projects and backups

| Setting | Default | Level |
|---|---|---|
| Projects folder | `~/Documents/Comilito`. *Move to…* moves every project to another folder (backups stay in the app data folder, decision 0026); *Choose…* only changes where Comilito looks (decision 0023) | this computer |
| Open the last project at start | off (also a switch on the home screen) | this computer |
| Window size, position and maximised state | remembered from the last run (1280×800, centred, the first time) | this computer, in `.window-state.json` next to `preferences.json` |
| Backup interval | 30 min | default for new projects |
| Backups to keep | 1 | default for new projects |
| Include audio in backups | off | default for new projects |

### Behaviour

| Setting | Default |
|---|---|
| When adding a scene that's already in the stack | ask (other choices: always move, always duplicate) |
| When removing a sound | ask (other choices: always move to trash) |
| When loading a preset over an unsaved tapestry | ask (other choice: always replace without saving) |
| Closing a window with unsaved changes (✕ or Esc) | ask (other choices: save, discard). The window's Cancel button always discards. |
| Warn when quitting during hashing or backup | on (not built yet) |

### Shortcuts

| Setting | Default |
|---|---|
| Remove the top scene from the tapestry | Backspace (any key except Space, Esc and /; or none) |

### Appearance

| Setting | Default |
|---|---|
| Theme | follow system (light / dark) |

## Project settings window

Shows the same **Levels** and **Sound defaults per category** sections, plus **Backups**, but for
the open project. These values are stored in `comilito.json` under `defaults` and `backups`.

## Data

App `preferences.json` (`src-tauri/src/prefs.rs`), with its defaults:

```json
{
  "formatVersion": 1,
  "projectsFolder": null,
  "openLastProject": false,
  "recentProjects": ["/home/me/Documents/Comilito/Curse of Strahd"],
  "playback": { "outputDevice": null, "pauseFadeMs": 300, "stopFadeMs": 1000 },
  "behaviour": {
    "duplicateInStack": "ask",
    "removeSound": "ask",
    "unsavedChanges": "ask",
    "replaceUnsavedTapestry": "ask"
  },
  "shortcuts": { "removeTopScene": "Backspace" },
  "appearance": {
    "mode": "system",
    "theme": "codex",
    "lightTheme": "codex",
    "darkTheme": "ironwood",
    "accent": "theme",
    "customAccent": "#7a4fb5",
    "capitals": true,
    "watermark": true,
    "rules": true,
    "blackletter": true
  },
  "newProjectDefaults": { "...": "masterVolume, categories, defaults and backups, as in a project" }
}
```

- `projectsFolder: null` means the default, `~/Documents/Comilito`.
- `playback.outputDevice`: the chosen output's id (for example
  `pulseaudio:alsa_output.usb-…-stereo`), or `null` for the system default.
- `shortcuts` values are `KeyboardEvent.key` names (`"Backspace"`, `"Delete"`, `"x"`); `""` turns
  the shortcut off.
- `appearance.mode` is `"system"` (follow the desktop's light/dark, using `lightTheme` /
  `darkTheme`) or `"fixed"` (always `theme`). `accent` is `"theme"`, `"desktop"` or `"custom"`
  (then `customAccent` is used). See [themes.md](themes.md).
- Missing fields get their defaults, so older files still load. Files from before themes existed
  had `appearance.theme` set to `"system"`, `"light"` or `"dark"`; these are migrated on load.

Project `comilito.json`:

```json
"defaults": {
  "duckLevel": 0.3,
  "transitionMs": 2500,
  "soundVolume": 1.0,
  "categories": {
    "music":    { "loop": true,  "fadeInMs": 2000, "fadeOutMs": 2000, "shuffle": false },
    "ambience": { "loop": true,  "fadeInMs": 1500, "fadeOutMs": 1500 },
    "effects":  { "loop": false, "fadeInMs": 0,    "fadeOutMs": 0 }
  },
  "normalize": { "music": true, "ambience": true, "effects": false, "targetLufs": -18 }
}
```

In scenes and sounds, `duckLevel`, `transitionMs`, `volume`, `loop`, `fadeInMs` and `fadeOutMs`
become **optional**: if a field is missing, the project default is used.
