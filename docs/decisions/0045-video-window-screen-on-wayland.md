# 0045. Put the video window on its screen through fullscreen on Wayland

- Status: Accepted (amends [0035](0035-video.md): choosing the video window's screen)
- Date: 2026-11-16

## Context

Choosing a screen for the video window moved it with `set_position`, then made it fullscreen
if it was. On Wayland (the owner's desktop, and most Linux desktops now) apps can't place their
own windows: the compositor ignores the request, so the window stayed where it was, and
fullscreen happened on that same screen. Wayland also names a laptop's own screen "Unknown".

## Options considered

- **Force X11 (XWayland) for the app:** moving works, but the whole app loses native Wayland
  (scaling, input) for one feature.
- **Fullscreen on the chosen screen:** Wayland lets an app ask for fullscreen on a given screen
  (`xdg_toplevel.set_fullscreen` with an output), and GTK exposes it as
  `gtk_window_fullscreen_on_monitor`, which also works on X11. Tauri's `set_fullscreen(true)`
  doesn't take a screen, so this calls GTK directly, through the `gtk` crate Tauri already
  builds on (`WebviewWindow::gtk_window`).
- **Do nothing on Wayland:** the picker would keep silently failing.

## Decision

- Fullscreen always goes to the chosen screen: on Linux through GTK on the main thread,
  elsewhere by moving the window there first as before. Turning fullscreen on with F or the
  button uses the chosen screen too, not just the one the window happens to be on.
- A window that isn't fullscreen is still moved with `set_position` where that works (X11,
  Windows, macOS). On Wayland `set_video_screen` reports that it couldn't move it, and the
  window says to drag it there or press F. Preferences › Video says the same.
- Screens are numbered in the OS's order, which on Linux is GDK's monitor numbering (what GTK's
  call takes). A screen with no name, the name "Unknown", or a name another screen shares is
  called "Screen N", so every name means one screen.

## Consequences

- The `gtk` crate (0.18, Linux only) is a direct dependency; it was already compiled in.
- A screen saved as "Unknown" before this no longer matches; the main screen is used until
  another one is chosen.
- Untested on Windows and macOS (as all of the video window, see TODO.md).
