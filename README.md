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

## Sharing and saving

A whole setup can be exported as a single `.comilito` bundle and opened on another computer:

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

Planning. No code yet.

## Development

> To be filled in once the project is scaffolded.

Prerequisites (planned):

- Rust (via [rustup](https://rustup.rs))
- Node.js 20+
- Tauri's [Linux system dependencies](https://tauri.app/start/prerequisites/)

## Acknowledgements

Inspired by [Ascapes Mixer](https://github.com/ilyaly/ascapes-mixer) by ilyaly (MIT).
Comilito is an independent implementation, not a fork.
