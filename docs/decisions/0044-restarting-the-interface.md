# 0044. Restart the interface by reloading the webviews

- Status: Accepted
- Date: 2026-11-16

## Context

The owner asked for an option and a shortcut to restart the UI: unforeseen bugs may leave the
screen stuck or wrong, and quitting and reopening Comilito in the middle of a session (and
finding the project again) is slow.

## Options considered

- **Restart the whole app** (Tauri's `restart`): a truly fresh start, but the sound stops, the
  project closes and reopens, and downloads and conversions are interrupted.
- **Reload the webviews:** everything on screen is rebuilt, while Rust (which owns the project,
  the audio, downloads and conversions, decision 0002) carries on untouched. A Rust-side bug
  isn't fixed by it, but those are rarer and the app can still be quit.

## Decision

Reload the webviews. *Restart the interface* reloads the main window and tells the video
window to reload too; both rebuild themselves from Rust's state like at start, so the project
stays open and the sound keeps playing.

- Shortcut: F5 by default (`shortcuts.restartInterface`, changeable or off in Preferences ›
  Behaviour › Shortcuts). As the way out of a stuck screen it's handled before everything
  else: with a dialog open, and in a text field when the key doesn't type anything.
- Buttons in Preferences › Projects & tools › Troubleshooting (disabled while Preferences has
  unsaved changes) and in About.
- Unsaved changes in an editor (a sound, a scene) are asked about first, as when leaving it.
- In the video window the shortcut reloads only that window.

## Consequences

- What was only on screen is lost: the Library's selection, an open dialog, unsaved changes in
  a settings window other than Preferences (Project settings, the theme editor) when restarted
  with the shortcut.
- The view after a restart is the Tapestry (or the home screen), like at start.
