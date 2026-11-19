# CLAUDE.md

Guidance for Claude (and other AI assistants) working on Comilito.

## Project

Comilito is a Tauri 2 desktop soundboard/mixer for tabletop RPG game masters (D&D and others), with three sound categories:
Music, Ambience and Effects. Read [README.md](README.md) and [docs/](docs/) before making changes.

## Who you're working with

The owner knows C, C++, C#, Python, Go and Bash, and is **new to Rust, Tauri and Svelte**.
Understanding the code matters more than writing it fast.

- Explain Rust-specific concepts (ownership, borrowing, lifetimes, `Result`/`?`, traits,
  `Arc`/`Mutex`, async) the first time they show up, comparing them to C++/C#/Go where that helps.
- Prefer clear, explicit code over clever code. Avoid macros-heavy or overly generic abstractions.
- Comments explain *why*, not *what*.
- Keep changes small and reviewable. One concept per step when introducing something new.

## Architecture rules

- **All audio playback happens in Rust** (kira). The webview never plays audio — no `<audio>`,
  no Web Audio API, no Howler. The UI sends commands; Rust plays sound. See
  [docs/decisions/0002-audio-in-rust.md](docs/decisions/0002-audio-in-rust.md).
- The frontend talks to Rust only through Tauri commands and events. Keep the command surface
  small and documented in [docs/architecture.md](docs/architecture.md).
- A project is a folder (`comilito.json` + `audio/`). File paths in `comilito.json` are **relative** to the project folder.
- The save format is versioned (`formatVersion`). Any change to it must update
  [docs/save-format.md](docs/save-format.md) and keep older files loadable (or migrate them).
- Linux is the primary target; don't introduce anything that breaks Windows. Test on Linux first.

## Dependencies

Don't add a dependency (crate or npm package) without saying why it's needed and whether
it's small, maintained and cross-platform. Prefer the standard library and Tauri's official plugins.

## Documentation and decisions

- [CHANGELOG.md](CHANGELOG.md) lists user-visible changes. Add an entry under **Unreleased**
  (Added / Changed / Fixed) with each change a user would notice.
- `docs/` describes how the app works. Update it when behaviour changes.
- `docs/decisions/` holds decision records (ADRs). When a meaningful decision is made in
  conversation (tech choice, format, trade-off), write a new numbered record using the
  template in [docs/decisions/README.md](docs/decisions/README.md) and add it to the index.
  Don't rewrite old decisions — supersede them with a new one.

## Git

- Commit in small, logical steps with clear messages (imperative mood: "Add mixer track model").
- Don't commit generated output (`target/`, `node_modules/`, `dist/`, build artifacts) or audio test files larger than a few hundred KB.
  Exception: the bundled sample audio in `src-tauri/resources/sample/`, re-encoded to keep it
  small (a few MB in total) and credited in [CREDITS.md](CREDITS.md) with its licence.
- Do not push unless asked.


## TODO.md

In this project there is a TODO.md where the developer will write things to correct. When asked, look into it and implement the requests therein.

- Do not implement new features without permission.
- Implement fixes for bugs freely.
- If you find other things left to be done, add them there.
- You can format the document, but only remove sentences if they are fixed.
- Do not remove headers even if they have no content

## Testing the running app

The UI is driven through **WebDriver**: `tauri-driver` (cargo) talks to `WebKitWebDriver`
(apt package `webkit2gtk-driver`), which launches the app and controls its webview — find
elements, click, type, read text, run JS, screenshot the window. WebDriver is plain HTTP/JSON
on `localhost:4444`, so `curl` is enough; no test framework is added to the project. See
[docs/decisions/0036-ui-testing-with-webdriver.md](docs/decisions/0036-ui-testing-with-webdriver.md).

**Ask the user first** before anything that opens the app on their desktop and takes control of
the mouse or keyboard, or takes a screenshot (of the screen or the app window). Say what the test
will do. This doesn't apply to unit tests, type-checks or a headless (Xvfb) run.

WebDriver only reaches the webview: native dialogs (file pickers, `rfd` message boxes) can't be
driven. `~/.cargo/bin` may not be on `PATH` in the agent's shell; call `~/.cargo/bin/tauri-driver`.

### Build

`tauri-driver` needs a binary with the frontend bundled in (a plain `cargo build` loads the
Vite dev server instead):

```bash
npm run tauri build -- --debug --no-bundle   # → src-tauri/target/debug/comilito
```

### Wayland (the default on the owner's desktop: Pop!_OS, COSMIC)

The app runs natively on Wayland (GTK picks it whenever `WAYLAND_DISPLAY` is set).

```bash
~/.cargo/bin/tauri-driver &      # listens on localhost:4444
APP=$PWD/src-tauri/target/debug/comilito
SID=$(curl -s -X POST localhost:4444/session -H 'Content-Type: application/json' \
  -d "{\"capabilities\":{\"alwaysMatch\":{\"tauri:options\":{\"application\":\"$APP\"}}}}" \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["value"]["sessionId"])')

curl -s localhost:4444/session/$SID/title                       # {"value":"Comilito"}
curl -s -X POST localhost:4444/session/$SID/element -H 'Content-Type: application/json' \
  -d '{"using":"css selector","value":"button.some-class"}'     # → element id
curl -s -X POST localhost:4444/session/$SID/element/<id>/click -d '{}'
curl -s -X POST localhost:4444/session/$SID/execute/sync -H 'Content-Type: application/json' \
  -d '{"script":"return document.body.innerText","args":[]}'
curl -s localhost:4444/session/$SID/screenshot                  # base64 PNG of the webview (ask first)
curl -s -X DELETE localhost:4444/session/$SID                    # closes the app
kill %1                                                          # stop tauri-driver
```

A full-screen screenshot (to see the video window, native dialogs, decorations):
`cosmic-screenshot --interactive=false --modal=false --notify=false -s <dir>` (ask first).

The app's stderr goes to `~/.local/share/com.comilito.app/logs/stderr.log` when it isn't a
terminal, so GTK/WebKit messages (and `WAYLAND_DEBUG=1` output) end up there.

### X11

On the owner's desktop, force X11 through XWayland — same commands, started with:

```bash
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 ~/.cargo/bin/tauri-driver &
```

Headless, on a virtual X display (no window on the user's desktop, no permission needed):

```bash
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 xvfb-run -a -s '-screen 0 1600x1000x24' ~/.cargo/bin/tauri-driver &
```

- WebDriver's `/screenshot` hangs on the virtual display. Capture the display instead:
  `XAUTHORITY=$(ls /tmp/xvfb-run.*/Xauthority) DISPLAY=:99 import -window root shot.png`
  (ImageMagick; `xvfb-run -a` picks `:99` first), then crop with `convert … -trim`.
- The app uses the real config, data and projects folders. To keep the owner's preferences,
  window state and projects untouched, start `tauri-driver` with `HOME`, `XDG_CONFIG_HOME`,
  `XDG_DATA_HOME` and `XDG_CACHE_HOME` pointing into a scratch folder (create
  `$HOME/Documents` there), and make a project with
  `window.__TAURI_INTERNALS__.invoke("create_project", {name: "Test", sample: true})`.

Use X11 for anything Wayland doesn't allow apps to do (e.g. setting their own window position)
and to check that X11 users aren't broken; Wayland is what most Linux users will run.

## Pencil

- When using pencil to generate changes, before creating any code, confirm if the design is correct.