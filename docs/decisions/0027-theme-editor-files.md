# 0027. The theme editor can add a woodcut, font files and licences

- Status: Accepted (extends 0025)
- Date: 2026-11-13

## Context

Decision 0025 let themes carry a watermark, fonts and licences, but only through importing a
`.comilito-theme` package. The theme editor could keep those files but not add or change them,
so making such a theme meant zipping it by hand.

## Options considered

- **Upload when saving:** the editor remembers picked paths and sends them with the theme on
  Save. One command, but a save would then fail late (a bad file found only at the end), and
  the live preview couldn't show a picked font or woodcut before saving.
- **Store on pick:** each picked file is checked and stored right away (same checks as an
  import) and the draft refers to its stored name. The preview works at once; a file picked and
  then dropped is left unused until cleanup.

## Decision

Store on pick, through one new command, `add_theme_file(path, role)` (role: watermark, font or
licence), which returns the stored name. Saving a theme also removes stored files no saved
theme uses, so files picked in a cancelled edit don't pile up.

In the editor:

- **Woodcut:** a preview painted in the theme's gold, "Choose image…" and "Use the horse".
- **Font files:** "Add font file…" asks for the name to show it under (guessed from the file
  name) and makes it the display font; each font can be removed. Removing the font in use falls
  back to the base theme's font.
- **Licences:** "Add licence…" and remove.

## Consequences

- Themes with their own fonts and woodcut can be made entirely in the app, then exported.
- One more command; the file checks are shared with import, so there's one set of rules.
