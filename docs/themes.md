# Themes

> Status: accepted ([decision 0016](decisions/0016-themes-and-desktop-accent.md), packages
> [decision 0025](decisions/0025-theme-packages.md)).

A theme is a set of design tokens: colours, a display font, a body font, and whether it's light
or dark. The three built-in themes (Codex, Ironwood, Hearth) live in `src/lib/themes.ts`. User
themes are saved by Rust (`src-tauri/src/themes.rs`) in the app's config folder.

A user theme can also bring files with it:

- **a watermark**, drawn instead of the skeletal horse. Only its shape is used: the app paints it
  in the theme's *gold* colour, so it suits the theme even after the colours are edited. A
  one-colour PNG with a transparent background works best.
- **fonts**, which the display or body font can then use.
- **licence texts** for those files (fonts usually require their licence to travel with them).

## Where themes are stored

```
<config>/themes/
  user-<id>.json                 one file per theme
  assets/
    <sha256>.png                 files themes brought, named by their SHA-256
    <sha256>.woff2
    <sha256>.txt
```

`<config>` is `~/.config/comilito` on Linux and `%APPDATA%\comilito` on Windows.

In a stored theme, `watermark`, `fontFiles` and `licences` hold those stored names. A copy of a
theme points at the same files. When a theme is saved or deleted, files no theme uses are
deleted.

## In the theme editor

Under the colours ([decision 0027](decisions/0027-theme-editor-files.md)):

- **Woodcut:** a preview of the watermark painted in the theme's gold. *Choose image…* picks a
  PNG, WebP or SVG; *Use the horse* goes back to the skeletal horse.
- **Font files:** *Add font file…* picks a WOFF2, WOFF, TTF or OTF, asks what to call it (guessed
  from the file name, e.g. `NewRocker-Regular.woff2` → "New Rocker") and makes it the display
  font. The theme's fonts are listed first in both font menus. Removing the font in use switches
  back to the base theme's font.
- **Licences:** *Add licence…* attaches a licence text (e.g. a font's `OFL.txt`) so it travels
  with the theme when it's exported.

A picked file is checked and stored straight away (`add_theme_file`), with the same checks as an
import, so the whole app previews it before the theme is saved.

## Sharing: `.comilito-theme` packages

**Export…** in Preferences › Appearance writes a `.comilito-theme` file, and **Import theme
file…** reads one. It's a zip:

```
theme.json                       the theme; file references are paths inside the zip
watermark.png                    optional (PNG, WebP or SVG)
fonts/NewRocker.woff2            optional (WOFF2, WOFF, TTF or OTF)
licences/licence-1.txt           optional
```

`theme.json` looks like this. Everything after `fonts` is optional, and `id` is ignored (an
imported theme always gets a new one, so importing never overwrites anything):

```json
{
  "id": "",
  "name": "Horde",
  "dark": true,
  "colors": { "bg": "#1c1412", "accent": "#d0301f", "...": "every token in COLOR_TOKENS" },
  "fonts": { "display": "New Rocker", "body": "Inter" },
  "watermark": "watermark.png",
  "fontFiles": { "New Rocker": "fonts/NewRocker.woff2" },
  "licences": ["fonts/OFL.txt"]
}
```

`fonts.display` and `fonts.body` name either a font bundled with the app (see `DISPLAY_FONTS`
and `BODY_FONTS` in `src/lib/themes.ts`) or a family in `fontFiles`. Colour tokens the theme
leaves out are taken from Codex (light) or Ironwood (dark).

Plain `.json` themes (the earlier export format) still import, as long as they don't refer to
files.

### What import checks

A theme file is something people download and pass around, so import is strict:

- colours must be `#rgb` or `#rrggbb`, and font names only letters, digits, spaces, `-` and `_`;
- every file the theme names must be in the package, of a type allowed for its role, and its
  first bytes must match its extension (a "PNG" that's really an HTML page is refused);
- `theme.json` is limited to 256 KB, each file to 4 MB, and the whole package to 16 MB, counted
  on the bytes actually read rather than the sizes the zip claims;
- files are looked up by the names `theme.json` gives and stored under their hash, so no path
  from the zip is ever used to write to disk;
- an SVG watermark is only ever used as a CSS mask, where scripts inside it don't run.

## How the webview gets the files

`read_theme_asset(name)` returns a stored file's raw bytes (an `ArrayBuffer` in JavaScript).
`loadThemeFiles` in `src/lib/themes.ts` turns fonts into `FontFace`s and the watermark into a
`blob:` URL, caching both. That avoids Tauri's asset protocol, which would let the webview read
a folder on disk.

## Extra themes

`extras/themes/` holds optional themes that aren't bundled with the app. Each is a folder laid
out like a package. `extras/themes/build.sh` zips them into `extras/themes/dist/`. See
[extras/themes/README.md](../extras/themes/README.md).
