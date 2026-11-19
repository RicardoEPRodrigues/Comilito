# 0048. Scale the interface with the webview's zoom

- Status: Accepted
- Date: 2026-11-18

## Context

Some screens (a laptop across the table, a big monitor far away, tired eyes late in a session)
need the interface bigger or smaller: the text and everything else with it. Comilito has no
setting for this; the desktop's display scale applies to every app at once.

An earlier note (decision 0034) warned that zooming the webview would move the drop point of
files dragged in from the file manager on Linux, so drag and drop had to be checked.

## Options considered

- **The webview's own zoom** (`Webview::setZoom`, what Ctrl + does in a browser): the whole page
  is laid out as if the window were smaller (or larger) and drawn bigger (or smaller). Text,
  icons, borders and spacing all follow; the layout reflows to fit. Inside the page every
  coordinate stays consistent: `clientX`, `getBoundingClientRect` and `elementFromPoint` all
  use the zoomed CSS pixels, so the app's own HTML5 drag and drop (decision 0014), context
  menus and the waveforms need no change. Only positions that come from outside the page in
  window pixels need dividing by the zoom: Tauri's native file drop on Linux (decision 0034).
  Needs one permission, `core:webview:allow-set-webview-zoom`.
- **CSS `zoom` on the root element:** looks the same, but WebKit has long reported mouse
  positions and element boxes in different spaces under CSS zoom (varying by version), which
  would break hit-testing for drag and drop and the waveform. Too risky across WebKitGTK,
  WebView2 and WKWebView versions.
- **Scaling the root `font-size`** (rem units): only what's sized in `rem` grows. Much of the
  CSS uses `px` (borders, icons given a `size`, canvas sizes), so the result would be uneven
  and would need a sweep of every component.

## Decision

Use the webview's zoom. `appearance.scale` in `preferences.json` (this computer, default 1.0,
kept within 0.5–2.0) is applied to the main window whenever it changes, live while Preferences
is open (like the theme). Preferences › Appearance › Interface size offers steps from 50% to
200%; Ctrl + / Ctrl − step through them and Ctrl 0 goes back to 100%, saving straight away
(not while Preferences is open, whose draft would overwrite them). The video window isn't
scaled: it only shows the picture.

The Linux native drop position is divided by the zoom (`FileDropImport.svelte`).

## Consequences

- No component needs to know about the scale. New code that gets a position from Rust or the OS
  in window pixels must divide it by `zoom` (`src/lib/scale.ts`).
- At large scales the page is effectively narrower than the window (a 640 px wide window at 200%
  lays out like 320 px), so the narrow-window problems noted in TODO.md show up sooner.
- The native drop position on Windows and macOS isn't used (the page's drag events are), so the
  zoom changes nothing there; still untested on those platforms (as the drop itself).
