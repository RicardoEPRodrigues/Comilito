# 0018. A full-screen Library workspace for editing sounds

- Status: Accepted
- Date: 2026-11-13
- Amends: [0017](0017-library-drawer-and-presets-tab.md) (in-place sound editor)

## Context

Editing a sound in place (screen F) worked, but the Scenes/Presets panel and the effects pad
took space without helping. With hundreds of sounds, managing the library (folders, tags,
missing files, bulk changes) needs its own room.

## Decision

The Library has a **workspace** of its own that fills the window. It's opened by:

- the maximise button on the Library drawer
- clicking **Library** in the breadcrumbs
- a shortcut

The Tapestry keeps playing (the mixer bar stays). **Back** or **Esc** returns to the Tapestry.

Layout (screen F in `pencil/comilito.pen`):

- **Left, folder tree:** smart lists (All sounds, Favourites, Recent, Unused, Missing files), then
  the categories, each with its folders (the real subfolders of `audio/<category>/`). There's a
  New folder action, Add files / From URL / Open in file manager buttons, and the downloads queue.
- **Middle, sound list:** the open folder's sounds with preview, duration, loudness, tags and
  favourite, and sorting. Shift or Ctrl-click selects several for bulk editing (folder, tags,
  fades, normalization).
- **Right, sound editor:** the in-place editor from 0017, plus **Source** (for downloaded audio)
  and **Used in** (scenes and presets, with links), and *Remove from project…*.
- **Open in file manager** is available for the whole library and for each folder. It uses
  Tauri's opener plugin (`revealItemInDir` / `openPath`).

The Scenes/Presets panel and the effects pad aren't shown in this workspace.

## Consequences

- Two "views" in the main window: Tapestry (play) and Library (manage). Play/Edit mode still
  applies; editing in the Library workspace requires Edit mode.
- Bulk editing needs a multi-value state in the editor (for example, "Fade out: mixed").
- The drawer remains the quick way to find and drag sounds during play.
