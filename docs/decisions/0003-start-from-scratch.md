# 0003. Start from scratch instead of forking Ascapes Mixer

- Status: Accepted
- Date: 2026-11-12

## Context

[Ascapes Mixer](https://github.com/ilyaly/ascapes-mixer) (MIT licence) is the inspiration for
Comilito and already uses Tauri 2 + Svelte 5, so forking it was an option.

Checked on 2026-11-12: the repo has about 93 KB of Svelte and about 1 KB of Rust (Tauri boilerplate
only). All audio runs in the webview, and the last push was in March 2025.

## Options considered

- **Private fork:** a working UI from day one. But the audio core (the most important part)
  would have to be replaced to follow decision 0002. We'd be learning someone else's
  structure instead of our own, and their save format would come along with it.
- **From scratch, with Ascapes as a reference:** slower start, but every line is understood,
  the architecture is built around a Rust audio engine from the beginning, and the save format
  is our own.

## Decision

Start from scratch. Use Ascapes Mixer only as a reference for features and UX ideas.

## Consequences

- No inherited code, so no licence obligations. If any code is copied later, keep the MIT notice.
- The UI takes longer to reach feature parity.
- The README credits Ascapes Mixer as inspiration.
