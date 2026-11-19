# 0049. A mini player in the main window, also used by narrow windows

- Status: Accepted
- Date: 2026-11-19

## Context

During a session the DM often has something else on screen: a virtual tabletop, notes, a
character sheet. The owner wants a very small Comilito that stays on top of those and shows what
matters while playing: the Tapestry and the scenes and presets to change it.

Separately, a narrow window squashed the bottom bar and the three columns until they broke. A
phone layout was designed too (Pencil, "Later · Mobile"), but it's for when Comilito runs on
Android; on the desktop a narrow window should simply use the mini player.

The designs are I, I2 and I3 (mini player) and J2 (bottom bar at a mid width) in
`pencil/comilito.pen`.

## Options considered

- **The main window turns into the mini player:** one window, so nothing has to be kept in step
  between two; the same Svelte state drives both layouts. Going back needs the old size and
  place, which must survive a quit while small.
- **A second window** (like the video window, decision 0035): could sit next to the full one,
  but every change would have to reach both, and it's more code for a use (both at once) the
  owner didn't ask for.

For where the window's normal size is kept while small:

- **In memory only:** lost when the app quits in the mini player. Worse, the window-state
  plugin (decision 0024) saves the window's size at quit, so the app would start small and no
  longer know the size to grow back to.
- **In a file next to `preferences.json`:** survives a quit. Its presence also says the mini
  player is on, so the app starts in it again. Not in `preferences.json` itself: it's window
  state, written often, not a setting.

## Decision

- The main window becomes the mini player. The header's button (picture-in-picture icon)
  turns it on: the window shrinks to 380 × 640 and stays on top. In the mini player the pin
  turns "on top" off and on, and the arrows button goes back to the full window, at its old size,
  position and maximised state (only the size on Wayland, where apps can't place their windows).
- Rust only handles the window (`mini_player.rs`: `mini_player`, `set_mini_player`,
  `set_mini_on_top`). While the mini player is on, `.mini-player.json` in the config folder
  holds the full window's placement (physical pixels) and the pin; going back removes it.
- Layout (`MiniPlayer.svelte`): the title row (project, Video, pin, back), Tapestry and Effects
  tabs, and the controls at the bottom (play, stop, master; the top scene's Music, Ambience and
  Effects). The Tapestry tab shows the stack (`MiniTapestry.svelte`: enable, remove, reorder by
  dragging, middle-click) and below it a Scenes / Presets drawer (`MiniScenes.svelte`) that the
  chevron folds down to its header; it isn't resizable. A single click there adds a scene on top
  or loads a preset (in the full layout a click selects, so that takes a double-click); both can
  also be dragged into the stack. The Effects tab is the effects pad without the video effects.
  No editing: menus to edit or rename scenes and presets stay in the full window.
- Entering the mini player first leaves an in-place editor (asking about unsaved changes) and
  the Library workspace.
- A window narrower than 860 CSS pixels (so the interface scale, decision 0048, counts) shows
  the same layout, but doesn't go on top and has no pin or back button: widening it brings the
  full layout back. Not while an editor or the Library workspace is open, whose unsaved work
  would be lost; those stay until closed. The window's minimum size is now 360 × 420.
- Between 860 and 1100 pixels the full layout stays, and the bottom bar drops its keyboard hint
  and lets its sliders share the width (design J2).
- With no project open (started in the mini player with "open the last project" off), the
  window goes back to full size: the home screen has no way out of a small on-top window.

## Consequences

- One layout to maintain for both "small on purpose" and "narrow by accident".
- The phone layout is still to do; it lives in the Pencil file under "Later · Mobile".
- Dialogs and menus open inside a 380 px window; they're already limited to the window's width.
- Untested on Windows and macOS: always-on-top and restoring the placement go through Tauri's
  window API there too, but haven't been tried.
- If the app is killed (not quit) in the mini player, the window-state plugin doesn't save the
  small size; it then starts with the mini player layout in a big window, and the arrows button
  still goes back normally.
