# 0036. Test the running app with WebDriver, on the real Wayland desktop

- Status: Accepted
- Date: 2026-11-15

## Context
Changes were checked by type-checks and unit tests, and the running app only on a virtual X11
display. The owner's desktop is Wayland (Pop!_OS, COSMIC), and the app runs on it natively, so the
X11-only runs never exercised what users will mostly see. Wayland doesn't let one program fake
input into another or read other windows the way X11 does (`xdotool`, `import`), so the usual X11
tools don't work there.

## Options considered
- WebDriver (`tauri-driver` + `WebKitWebDriver`): drives the webview by selector — click, type,
  read the DOM, screenshot the window. Works on Wayland and X11, doesn't move the user's real
  mouse, robust to layout changes. Can't reach native dialogs. Both tools are official
  (Tauri / WebKitGTK), the driver's version matches the system WebKit.
- Synthetic input (`wtype` for keys, `ydotool` via `/dev/uinput` for the mouse): drives anything
  on screen, native dialogs included, but takes over the user's real mouse and keyboard, clicks by
  pixel position, and COSMIC has no virtual-pointer protocol, so the mouse needs `ydotool` 1.x
  built from source.
- Keep using only a virtual X11 display: no interference with the user, but never tests Wayland.

## Decision
WebDriver, spoken to with `curl` (no test framework added to the project), on the real desktop
by default, with X11 (via XWayland, or headless with Xvfb) as a secondary check.
`cosmic-screenshot` covers whole-screen screenshots. The agent asks the user before running a
test that takes over the desktop or takes a screenshot.

## Consequences
- UI changes can be checked in the running app on Wayland.
- Native dialogs (file pickers, message boxes) still need checking by hand.
- Tests need a debug build with the frontend bundled (`npm run tauri build -- --debug --no-bundle`).
- If tests grow beyond quick checks, a WebDriver client (e.g. WebdriverIO) may be worth adding as
  a dev dependency — a separate decision.
