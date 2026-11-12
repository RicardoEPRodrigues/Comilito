# CLAUDE.md

Guidance for Claude (and other AI assistants) working on Comilito.

## Project

Comilito is a Tauri 2 desktop soundboard/mixer for D&D DMs, with three sound categories:
Music, Ambience and Effects. Read [README.md](README.md) and [docs/](docs/) before making changes.

## Who you're working with

The owner knows C, C++, C#, Python, Go and Bash, and is **new to Rust, Tauri and Svelte**.
Understanding the code matters more than writing it fast.

- Explain Rust-specific concepts (ownership, borrowing, lifetimes, `Result`/`?`, traits,
  `Arc`/`Mutex`, async) the first time they show up, comparing them to C++/C#/Go where that helps.
- Prefer clear, explicit code over clever code. Avoid macros-heavy or overly generic abstractions.
- Comments explain *why*, not *what*.
- Keep changes small and reviewable. One concept per step when introducing something new.

## Architecture rules

- **All audio playback happens in Rust** (kira). The webview never plays audio — no `<audio>`,
  no Web Audio API, no Howler. The UI sends commands; Rust plays sound. See
  [docs/decisions/0002-audio-in-rust.md](docs/decisions/0002-audio-in-rust.md).
- The frontend talks to Rust only through Tauri commands and events. Keep the command surface
  small and documented in [docs/architecture.md](docs/architecture.md).
- A project is a folder (`comilito.json` + `audio/`). File paths in `comilito.json` are **relative** to the project folder.
- The save format is versioned (`formatVersion`). Any change to it must update
  [docs/save-format.md](docs/save-format.md) and keep older files loadable (or migrate them).
- Linux is the primary target; don't introduce anything that breaks Windows. Test on Linux first.

## Dependencies

Don't add a dependency (crate or npm package) without saying why it's needed and whether
it's small, maintained and cross-platform. Prefer the standard library and Tauri's official plugins.

## Documentation and decisions

- `docs/` describes how the app works. Update it when behaviour changes.
- `docs/decisions/` holds decision records (ADRs). When a meaningful decision is made in
  conversation (tech choice, format, trade-off), write a new numbered record using the
  template in [docs/decisions/README.md](docs/decisions/README.md) and add it to the index.
  Don't rewrite old decisions — supersede them with a new one.

## Git

- Commit in small, logical steps with clear messages (imperative mood: "Add mixer track model").
- Don't commit generated output (`target/`, `node_modules/`, `dist/`, build artifacts) or audio test files larger than a few hundred KB.
- Only commit or push when asked.
