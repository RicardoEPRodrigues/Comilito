# 0022. Import audio by dropping files onto the window

- Status: Accepted
- Date: 2026-11-13

## Context

The owner wants to drag audio files from the file manager onto the app and import copies of
them, after a confirmation, one file or several. Decision 0014 turned Tauri's native file-drop
handling off (`dragDropEnabled: false`) because on Windows it swallows the HTML5 drag events
the stack relies on.

With the native handling off, the webview's own drop event is all that's left, and it differs
by platform (tested 2026-11-13):

- **WebKitGTK (Linux)** hides dropped files from the page: `dataTransfer.files` is empty and
  `text/uri-list` reads as an empty string. Its default action for a dropped file is to open it,
  which replaced the whole app with a media player playing the file.
- **WebView2 (Windows) and WKWebView (macOS)** give the page `File` objects: the contents, but
  no path.

## Options considered

- **Native file drop everywhere:** gives paths, but breaks the stack's drag and drop on Windows (0014).
- **HTML5 drop everywhere:** doesn't work on Linux, the primary target.
- **Per platform:** native drop on Linux, HTML5 drop elsewhere. On Linux, wry only intercepts
  drags that carry a `text/uri-list` (files), so the app's own HTML5 drags are unaffected.

## Decision

Per platform:

- **Linux:** `tauri.linux.conf.json` turns `dragDropEnabled` on. The frontend listens to the
  webview's drag-drop event, which gives the files' paths, and imports them with `add_sounds`
  (the same command as *Add files…*).
- **Windows and macOS:** native drop stays off. The browser drop event gives `File` objects.
  Each one is read in the webview and sent to the new command `import_dropped_file` as the raw
  request body; the name, category and folder go in headers. Rust saves it into the project.
- **Everywhere:** every drag of files over the window is stopped (`preventDefault`), so a drop
  can never open the file in the webview. Only audio files are offered for import; others are listed as skipped.
- A dialog (`FileDropImport.svelte`) lists the files and asks for the category and folder
  (defaulting to where the Library is pointing) before anything is copied. Afterwards the
  Library opens on the new sounds.

## Consequences

- `tauri.linux.conf.json` repeats the main window's settings, because Tauri's platform files
  replace the `windows` list rather than merging it. Window changes must be made in both files.
- On Windows and macOS a dropped file passes through the webview's memory once. That's fine
  for sound files, but a multi-GB file would be slow.
- The Windows/macOS path is untested (TODO.md).
- 0014 still holds for Windows and macOS.
