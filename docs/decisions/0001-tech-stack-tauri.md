# 0001. Use Tauri 2 with Rust and Svelte

- Status: Accepted
- Date: 2026-11-12

## Context

Comilito needs to run on Linux first, then Windows, possibly macOS and Android later.
It should be lightweight (Electron was ruled out as too heavy), and the code must stay easy
for the owner to understand. Learning a new language is fine.

## Options considered

- **Tauri 2** (Rust + web UI): uses the OS webview, binaries of about 5–15 MB, supports desktop and
  Android. Rust is new to the owner, but the Rust part of this app is small.
- **Flutter** (Dart): strong on Android, but desktop Linux audio plugins are less reliable.
- **Avalonia** (C#): known language, but cross-platform audio needs extra native libraries.
- **Pure Rust GUI** (egui/Iced/Slint): smallest builds, but plainer UI and hard on Android.
- **Godot**: excellent audio, but awkward for ordinary app UI.
- **Electron**: rejected, too heavy.

## Decision

Tauri 2, with a Rust backend and a Svelte + TypeScript frontend.

## Consequences

- Two languages in the project: Rust (audio, files) and TypeScript/Svelte (UI).
- On Linux the UI runs in WebKitGTK, so the webview must not be relied on for audio (see 0002).
- Android is a realistic later target through Tauri 2's mobile support.
- The owner learns Rust along the way; code and explanations should support that (see CLAUDE.md).
