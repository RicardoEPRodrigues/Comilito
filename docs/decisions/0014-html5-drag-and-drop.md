# 0014. Hand-written HTML5 drag and drop for the stack

- Status: Accepted
- Date: 2026-11-13

## Context

Decision 0011 made the stack the focus of the UI, driven by drag and drop: scenes and presets
are dragged from the library to any position in the stack, entries are dragged to reorder,
and dragging an entry out removes it. 0011 left open whether to use a library.

## Options considered

- **A drag-and-drop library** (for example `svelte-dnd-action`): handles sorting animations
  and touch input, but adds a dependency, and its model (lists that own their items) doesn't
  match ours well, since Rust owns the stack and the UI only sends commands.
- **The browser's built-in HTML5 drag and drop events**, about 80 lines in `StackPanel.svelte`:
  no dependency, and the drop handler just calls `stack_insert` / `stack_move` / `preset_insert`.

## Decision

Use HTML5 drag and drop, written by hand. What's being dragged is kept in the UI store
(`ui.drag`) rather than in the event's `dataTransfer`, which some webviews make awkward to read
during a drag.

## Consequences

- Tauri's own file-drop handling must be off (`"dragDropEnabled": false` in `tauri.conf.json`),
  because on Windows it swallows HTML5 drag events. Files therefore can't be dropped onto the
  window from a file manager; sounds are added with the file picker.
- No touch support (HTML5 drag events don't fire for touch). Revisit if Android becomes a target.
