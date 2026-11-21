# 0065. The AppImage adds itself to the app menu, always asking first

- Status: Accepted
- Date: 2026-11-21

## Context

With the Flatpak dropped (decision 0064), the AppImage is Linux's only package. An AppImage is
a single file: it carries a `.desktop` launcher and icons inside, but nothing copies them out,
so Comilito never appears in the desktop's app menu or gets its icon in the dock. Tauri doesn't
do it. Tools like Gear Lever and AppImageLauncher do, but most users don't have them.

## Options considered

- **Leave it to those tools**: nothing to build, but most users get no menu entry.
- **Add it silently at every start**: no questions, but it writes into the user's folders, and
  re-points the entry, without consent.
- **Ask**: at start, when the entry is missing or out of date, offer to add or update it, with
  a way to stop asking; the same actions in Preferences.

## Decision

Comilito asks. `app_menu.rs` looks at `~/.local/share/applications` (`$XDG_DATA_HOME`) for
`me.colorcrow.comilito.desktop` and works out a state:

- **missing**: no launcher starts this AppImage: offer to add one;
- **moved**: ours starts a file that isn't there any more: offer to point it at this copy;
- **other copy**: ours starts another copy that still exists (a second download): offer to point
  it at this one;
- **changed**: this version writes the launcher or its icons differently: offer to update;
- **current**, or **elsewhere** (another tool's launcher already starts this file): nothing to ask.

**Every change asks first**, at start or in Preferences, removing included; only looking is
silent. The offer can also move the AppImage to `~/Applications` first (ticked when it's in
Downloads or `/tmp`, which get cleaned); a file of the same name there goes to the trash. The
offer's *Don't ask again* turns off `behaviour.askAppMenu`; Preferences › Projects & tools › App
menu turns it back on and adds, updates or removes the entry.

The launcher is named after the app ID, which is the window's Wayland app ID, and has
`StartupWMClass=comilito` (the X11 window class), so docks match the window to it. `TryExec`
names the AppImage, so a deleted AppImage disappears from the menu instead of leaving a launcher
that fails. Icons are copied from the AppImage (`$APPDIR/usr/share/icons/hicolor`) as
`me.colorcrow.comilito.png` in each size. Whether the entry is current is decided by comparing
the launcher's text and the icons byte for byte with what this version would write.

It only runs inside an AppImage (`APPIMAGE` set), on Linux, outside the Flatpak.

## Consequences

- A change to the launcher's text or the icons in a new version asks every user to update.
- Removing Comilito cleanly means *Remove…* before deleting the AppImage; otherwise `TryExec`
  hides the entry, but its files stay in `~/.local/share` (README, Uninstalling).
- Opening `.comilito` files from the file manager would need the app to accept a file at launch;
  not done.
