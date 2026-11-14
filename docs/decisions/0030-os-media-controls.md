# 0030. Register with the OS as a media player

- Status: Accepted
- Date: 2026-11-14

## Context

The DM wants to pause and resume the tapestry without switching to Comilito: from the media
keys, a headset button, or the desktop's media widget (GNOME's notification area, KDE's media
applet, the Windows volume flyout). For that, the OS must know Comilito is a media player.

Each OS has its own protocol: **MPRIS** over D-Bus on Linux, the **System Media Transport
Controls** on Windows, **Now Playing** (MPRemoteCommandCenter) on macOS.

## Options considered

- **Write MPRIS ourselves with `zbus`** (already a dependency): no new crate, but Linux only,
  and a few hundred lines of D-Bus interface code to maintain. Windows would need its own code
  through the `windows` crate.
- **`mpris-server`:** a tidy MPRIS implementation, but Linux only.
- **`souvlaki`:** one small API over all three OSes (MIT, maintained, used by several Rust
  players). On Linux it can use `zbus` (feature `use_zbus`), which is already in the build, so
  there's no libdbus system dependency; on Windows it uses the `windows` crate version Tauri
  already pulls in. New crates on Linux: `souvlaki` and `pollster`.

## Decision

`souvlaki` with `default-features = false, features = ["use_zbus"]`, in `media_controls.rs`:

- Play, Pause, Play/Pause and Stop from the OS do what Space and Esc do. Play and Pause only act
  when they change something. "Raise" brings the window to the front.
- Next, previous, seeking and volume aren't offered: a tapestry has no track list, and the
  volumes are the DM's.
- What the OS shows: the **top scene** of the tapestry as the title (the project's name when no
  scene is enabled), the **music track** playing as the artist, the **project** as the album,
  and whether the tapestry is playing, paused or stopped.
- A thread checks the state four times a second and only tells the OS about changes.
- If the OS has no media controls (no D-Bus session, say), it's logged and the app works as
  before.

## Consequences

- Media keys and headset buttons pause and resume the tapestry, even while another window is
  focused, as long as the desktop routes them to the active media player.
- Comilito appears in the desktop's media widget whenever it runs, even with no project open
  (as "Comilito", stopped).
- Windows and macOS compile against souvlaki but are untested; on Windows the controls are tied
  to the main window's handle.
