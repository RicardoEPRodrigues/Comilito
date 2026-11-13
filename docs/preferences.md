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
| Music / Ambience / Effects category volume | 80% / 60% / 100% |
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
| Audio output device | system default |
| Pause / resume fade (Space) | 0.3 s (a short fade avoids clicks) |
| Stop all fade (Esc) | 1 s |
| Start paused when opening a project | on |

### Projects and backups

| Setting | Default | Level |
|---|---|---|
| Projects folder | `~/Documents/Comilito`. *Move to…* moves every project and the backups to another folder; *Choose…* only changes where Comilito looks (decision 0023) | this computer |
| Open the last project at start | off (also a switch on the home screen) | this computer |
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
| Warn when quitting during hashing or backup | on |

### Appearance

| Setting | Default |
|---|---|
| Theme | follow system (light / dark) |

## Project settings window

Shows the same **Levels** and **Sound defaults per category** sections, plus **Backups**, but for
the open project. These values are stored in `comilito.json` under `defaults` and `backups`.

## Data (draft)

App `preferences.json`:

```json
{
  "formatVersion": 1,
  "projectsFolder": "~/Documents/Comilito",
  "openLastProject": false,
  "recentProjects": ["/home/me/Documents/Comilito/Curse of Strahd"],
  "playback": { "outputDevice": null, "pauseFadeMs": 300, "stopFadeMs": 1000, "startPaused": true },
  "behaviour": { "duplicateInStack": "ask", "removeSound": "ask", "unsavedChanges": "ask" },
  "appearance": { "theme": "system" },
  "newProjectDefaults": { "...": "same shape as a project's defaults + backups" }
}
```

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
