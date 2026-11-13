# Comilito

A lightweight desktop soundboard and mixer for Dungeons & Dragons Dungeon Masters.

Named after *Invincible*, the Lich King's theme from World of Warcraft.

## What it does

Comilito lets a DM build a library of sounds for a session and play them live,
organised into three categories:

- **Music** — background tracks (usually one at a time, with crossfades)
- **Ambience** — looping environmental layers (rain, tavern chatter, wind…) that can be stacked
- **Effects** — short one-shot sounds (door slam, dragon roar, spell cast)

Each category has its own volume, and each sound has its own volume, loop and fade settings.

## Using Comilito

1. **Create a project** (one per campaign). It's a folder in `~/Documents/Comilito/`.
2. Switch to **Edit** mode (top bar). In **Sounds**, add music, ambience and effect files; they're
   copied into the project. ▶ previews a sound; ✎ sets its volume, loop and fades.
3. In **Scenes**, create scenes: a music playlist, ambience layers (each with a volume and an
   optional role), effects for the pad, and optionally a hotkey.
4. Back in **Play** mode, drag scenes into the **stack** (or double-click them). Scenes higher up
   duck the ambience below them; the top-most scene with music plays its music.
   Tick a scene off to disable it without removing it; drag it out of the stack to remove it.
5. **Space** starts / pauses, **Esc** stops everything, scene hotkeys add or toggle a scene.
   Click a stack entry to see its layers' effective volumes and adjust them live.
6. **Save stack as preset** for stacks you reuse; drag a preset in, or right-click → Replace stack.

Everything saves automatically. See [docs/scenes.md](docs/scenes.md) for how the stack works.

## Sharing and saving

Each project is a plain folder in `~/Documents/Comilito/` with its settings (`comilito.json`)
and its own audio files. Put those folders in a synced location to use them on several devices.

A project can also be exported as a single `.comilito` file (a zip) and imported elsewhere:

- **With audio included** — one self-contained file, everything works on import.
- **Without audio** — a small file holding only the configuration. On import, Comilito
  asks you to point it at a folder (or individual files) for any audio it can't find.

See [docs/save-format.md](docs/save-format.md).

## Platforms

- Linux (primary)
- Windows
- macOS and Android (planned, not yet supported)

## Tech stack

- [Tauri 2](https://tauri.app) — app shell, uses the OS webview (no bundled browser)
- **Rust** — audio engine ([kira](https://github.com/tesselode/kira)) and file handling
- **Svelte + TypeScript** — user interface

See [docs/architecture.md](docs/architecture.md) and [docs/decisions/](docs/decisions/).

## Status

First version: projects, a sound library, scenes, the scene stack with ducking and roles,
presets, an effects pad, export/import, relinking missing audio, hashing and automatic backups.
Not yet: choosing the audio output device, and a warning when quitting during hashing.

## Development

Prerequisites:

- Rust (via [rustup](https://rustup.rs))
- Node.js 20+
- Linux system libraries (Debian/Ubuntu/Pop!_OS; other distros: see
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)):

  ```sh
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev
  ```

  `libasound2-dev` (ALSA) is for audio output.

Run:

```sh
npm install        # once, and after dependencies change
npm run tauri dev  # starts the app with hot reload for the UI
```

Other commands:

| Command | What it does |
|---------|--------------|
| `npm run check` | Type-check the Svelte/TypeScript code |
| `npm run tauri build` | Build a release binary and installers into `src-tauri/target/release/` |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run the Rust tests |

## Acknowledgements

Inspired by [Ascapes Mixer](https://github.com/ilyaly/ascapes-mixer) by ilyaly (MIT).
Comilito is an independent implementation, not a fork.
