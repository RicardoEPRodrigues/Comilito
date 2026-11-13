# 0024. Move the projects folder in the background; remember the window size

- Status: Accepted
- Date: 2026-11-13

## Context

Decision 0023 moved the projects folder inside a normal Tauri command. Tauri runs plain
(non-`async`) commands on the main thread, and the command also held the app-state lock while
copying, so a move across disks froze the window until it finished. The owner wants the app to
stay responsive.

The owner also wants a bigger default window, and ideally the size from the last run.

## Options considered

- **Background move:**
  - an `async` command awaiting a blocking task;
  - a plain command that checks the move, starts a `std::thread` and returns.

  The second matches how downloads already work (a worker thread plus events) and needs no
  async code.
- **Window size:**
  - save the size ourselves (listen for resizes, store it in `preferences.json`);
  - use `tauri-plugin-window-state`, Tauri's official plugin for exactly this.

  The plugin is small, maintained by the Tauri team and works on Linux, Windows and macOS.
  It also handles the cases a hand-written version would get wrong (maximised windows,
  positions on a monitor that's gone).

## Decision

- **Moving:** `move_projects_folder` checks the move (`relocate::plan_move`), closes an open
  project inside the folder, sets `moving_projects`, and starts a thread. The command returns
  straight away.
  - The thread copies without holding the state lock. It reports `projects-move-progress`
    (bytes done of the total, at most ~10 times a second) and finally `projects-moved` (how many
    folders moved, or the error).
  - The thread then locks the state again to save the new folder and reopen the project.
  - While `moving_projects` is set, opening, creating, importing, duplicating and deleting
    projects are refused.
  - The UI covers the app with a progress dialog (`MovingProjects.svelte`).
- **Window:** `tauri-plugin-window-state` restores size, position and maximised state (not
  visibility or decorations). The default for the first run is 1280×800, centred.

## Consequences

- Supersedes the note in 0023 that the app is unresponsive during a move.
- A move can't be cancelled halfway; a folder that's partly copied when an error happens is
  removed, and the error says how many folders already moved.
- New dependency: `tauri-plugin-window-state` (Rust side only, no JavaScript package).
  It writes `.window-state.json` in the app config folder.
- Window settings live in both `tauri.conf.json` and `tauri.linux.conf.json` (see 0022).
