# 0005. Default storage in Documents, zip as the only format, lazy hashing

- Status: Accepted, point 2 superseded by [0006](0006-project-folders.md)
- Date: 2026-11-12
- Resolves the open questions in [0004](0004-save-format.md)

## Context

Decision 0004 left three questions open: where imported audio lives by default, whether to
also allow a plain JSON save, and when to compute file hashes.

## Decision

1. **Default storage is `~/Documents/Comilito/`** (on Windows, the user's Documents folder,
   resolved through the OS rather than hard-coded). It's easy to find, browse, edit by hand or
   delete. App-internal folders (`~/.local/share`, `%APPDATA%`) are hidden from most users, so
   they're used only for app settings, not user content.
2. **The `.comilito` zip is the only save format.** No plain `.comilito.json` alternative.
   There's one loader, one writer and one schema to maintain.
3. **Hashes are computed lazily in the background.**
   - The app stays fully usable while hashing runs.
   - The UI shows hashing is happening (for example a small status indicator with progress).
   - A hash is cached with the file's size and modification time, so it's computed once and
     only recomputed if the file changes.
   - If the user quits while hashing is in progress, the app warns them and offers to quit anyway.
     Unfinished hashes resume on the next launch.
   - If an export needs hashes that aren't ready, the export waits for those files and shows progress.

## Consequences

- The Rust side needs a background worker thread and a way to report progress to the UI (Tauri events).
- The quit flow must intercept the window close event to show the warning.
- Hashes are a matching aid, not a requirement: a sound without a hash can still be relinked by name and size.
