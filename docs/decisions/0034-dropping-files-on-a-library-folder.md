# 0034. Dropping files on a Library folder imports them there

- Status: Accepted
- Date: 2026-11-15

## Context

Dropping audio files onto the window ([0022](0022-dropping-files-onto-the-window.md)) opens an
import dialog whose category and folder default to where the Library is pointing, wherever the
files were dropped. The owner wants files dropped on a folder to go into that folder.

## Options considered

- **Drop straight into the folder, no dialog:** fastest, but a slip of the mouse copies files
  into the wrong place, and 0022 asked for a confirmation before anything is copied.
- **Keep the dialog, pre-filled with the folder under the pointer:** one extra click, nothing is
  copied by accident, and the folder can still be changed.

## Decision

- Keep the dialog. Its category and folder are those of the Library folder or category under the
  pointer when the files are dropped; elsewhere in the window, the folder the Library is showing
  (as before).
- Drop targets are marked in the markup with `data-drop-category` and `data-drop-folder`: the
  categories and folders in the Library workspace's left column, and the drawer's columns and
  folder groups (heading and sounds). `FileDropImport.svelte` finds the one under the pointer
  with `document.elementFromPoint`, so components need no file-drop code of their own.
- While files are dragged, the target is outlined (`ui.fileDropTarget`) and the overlay moves to
  the bottom of the window as "Drop into Ambience › Forest", so the outlined folder stays visible.
- The pointer comes from the HTML5 drag events on Windows and macOS (CSS pixels already), and
  from Tauri's native drag-drop events on Linux. Those are typed as physical pixels, but on
  Linux wry passes on GTK's widget coordinates, which are logical pixels: equal to CSS pixels at
  any display scale, as long as the webview isn't zoomed. They are used as they are. (Dividing
  them by `devicePixelRatio` put the point too high on a scaled display.) Once native events
  arrive, the page's own drag events are ignored except to stop the default action: WebKitGTK
  sends them too, with wrong positions, which made the outline flicker.

## Consequences

- Any other part of the UI can become a drop target by adding the two attributes.
- If the app ever zooms the webview, the Linux position must be divided by that zoom. (It does
  since decision 0048, the interface scale, and the position is divided.)
- If a Tauri update starts converting the Linux position to physical pixels, the outline will
  appear below the pointer on scaled displays.
