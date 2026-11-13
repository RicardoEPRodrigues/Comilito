# 0025. Themes can bring a watermark and fonts, shared as `.comilito-theme` packages

- Status: Accepted (supersedes the "user themes are JSON files" part of 0016)
- Date: 2026-11-13

## Context

The owner wanted two optional themes, themed on Warcraft's Horde and Alliance, that are not
bundled with the app but can be imported. Each needs its own woodcut in place of the skeletal
horse, and its own display font. Under 0016 a theme was a single JSON file of colours plus the
*names* of fonts bundled with the app, so it could carry neither an image nor a font.

## Options considered

- **Embed the files in the JSON as base64 data URLs:** one file, no format change beyond new
  fields. Clumsy with several fonts, about a third bigger, and hard to edit by hand.
- **A zip package (`theme.json` plus files), unpacked into a folder per theme:** simple to
  author, but a copy of a theme would need its own copy of every file, and the editor's
  "duplicate" would have to copy folders.
- **A zip package for sharing, with the files stored once, named by their SHA-256:** themes stay
  single JSON files that refer to stored files by hash. Copies share files for free, and the
  names can't point anywhere else on disk.

How the webview gets the files: Tauri's asset protocol (needs a scope that lets the webview read
part of the disk), or a command that returns the bytes.

## Decision

- A theme may have an optional `watermark`, `fontFiles` (family → file) and `licences`. Older
  theme files load unchanged.
- Themes are shared as **`.comilito-theme`** files: a zip with `theme.json` and the files it
  names. Export always writes one; import also accepts the old plain `.json`.
- On import, each named file is checked (type for its role, magic bytes, size limits) and stored
  in `<config>/themes/assets/<sha256>.<ext>`. When a theme is deleted, unused files are removed.
- The webview reads files through a new `read_theme_asset(name)` command (raw bytes), loads
  fonts with `FontFace` and shows the watermark from a `blob:` URL. No asset protocol.
- The watermark is used as a **mask painted in the theme's gold**, not shown in its own colours.
- For now this is import and export only. The theme editor keeps the files a theme came with,
  and offers its fonts in the font lists, but can't add or change them.
- The Horde and Alliance themes live in `extras/themes/` as package sources with a build
  script, and are not bundled. Their woodcuts are interpretations of the faction emblems, and
  Warcraft is Blizzard's trademark, so they're kept out of releases.

## Consequences

- The `zip` and `sha2` crates were already dependencies; nothing new is added.
- One more command in the API surface (`read_theme_asset`), documented in architecture.md.
- Theme fonts load asynchronously: the first frame after switching may briefly show the fallback
  font.
- Adding "choose a watermark / font file" to the theme editor later only needs a command that
  stores a picked file and returns its name.
- Font licences are carried along but not enforced; theme authors are responsible for them.
