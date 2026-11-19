# Extra themes

Optional themes that aren't bundled with Comilito. Import them from **Preferences › Appearance ›
Import theme file…**.

| Theme | Look | Display font | Woodcut |
|---|---|---|---|
| **Horde** (dark) | charred iron, blood-red header and accent, bronze ornaments | New Rocker | a tribal war sigil on a round iron shield |
| **Alliance** (light) | parchment, navy header, royal-blue accent, gold ornaments | Cinzel Decorative | a roaring lion in a kite shield, with fleur-de-lis and sword |
| **Hearth** (light) | parchment, dark header, side panel in the accent colour | Pirata One (bundled) | the skeletal horse |

Hearth was a built-in theme until [decision 0050](../../docs/decisions/0050-accent-following-themes.md);
its side panel follows the accent (desktop or custom). Horde and Alliance keep their own accent
whatever is chosen in Preferences (`"fixedAccent": true`), since their colours are built around
one faction colour.

The designs are in [pencil/extra-themes.pen](../../pencil/extra-themes.pen).

## Building the packages

Each folder is a theme package unzipped (the format is described in
[docs/themes.md](../../docs/themes.md)). To make the `.comilito-theme` files:

```bash
extras/themes/build.sh        # writes extras/themes/dist/<Name>.comilito-theme for each folder
```

`dist/` is not committed. After changing a woodcut, keep it a single colour with a transparent
background: the app only uses its shape and paints it in the theme's gold.

## Credits and licences

- **New Rocker** by Pablo Impallari, Brenda Gallo and Rodrigo Fuenzalida, and **Cinzel
  Decorative** by Natanael Gama. Both are under the SIL Open Font License 1.1; the licence travels
  with each theme (`fonts/OFL.txt`).
- The **woodcuts** were generated for this project in Pencil (pen.dev) and cleaned up to one
  colour on a transparent background.
- These are unofficial fan themes. *Warcraft*, the Horde and the Alliance are trademarks of
  Blizzard Entertainment; the woodcuts are interpretations of the faction emblems, not the
  official artwork. Keep these themes out of public releases of Comilito.
