# 0050. Theme colours that follow the accent; Birch replaces Hearth

- Status: Accepted
- Date: 2026-11-19

## Context

The accent can be the theme's, the desktop's or a custom colour (decision 0016), but only the
`accent` token changed with it. Codex's header is the same red as its accent, so with a blue
desktop accent the header stayed red next to blue highlights. Hearth's red side panel had the same
problem. Other themes (Horde, Alliance) are built around one colour, and any other accent looks
wrong in them.

The owner also wanted a clear light theme, like Ironwood but light, and Hearth out of the
built-in themes.

## Options considered

- **Per-theme code** (Codex's header is the accent): simple, but user and imported themes
  couldn't do the same.
- **A colour value meaning "the accent in use"**, stored in the theme like any colour: works for
  every theme, survives export and import, and the theme editor can offer it.
- For text on such a background: let the theme pick it (but it was picked for one accent and may
  not read on another), or **work it out from the accent** (near white or near black, like the
  text on accent buttons already is).

## Decision

- A colour token may be `"accent"`. `resolveColors` (`src/lib/themes.ts`) replaces it with the
  accent in use. Rust accepts `accent` besides `#rgb`/`#rrggbb` when saving or importing.
- When `header-bg`, `side-bg` or `mixer-bg` is `"accent"`, the text tokens on it
  (`header-text`; `side-text`, `side-muted`; `mixer-text`) are worked out from the accent.
- A theme with `"fixedAccent": true` always uses its own accent. The theme editor has a switch for
  it; Preferences says so when such a theme is chosen. Horde and Alliance have it.
- Codex's header is `"accent"`.
- New built-in light theme **Birch**: Ironwood's light sibling (cool white and pale greys, ember
  accent, Grenze Gotisch headings), clearer and brighter than Codex's parchment.
- **Hearth** moves to `extras/themes/hearth/` (Pirata One is bundled, so it needs no files), with
  its side panel following the accent. A computer that had Hearth chosen gets Codex (unknown theme
  ids already fall back to it).

## Consequences

- User themes copied from Codex before this keep a fixed red header (copies store every token);
  the editor's link button makes it follow.
- Theme cards in Preferences preview each theme with the accent in use.
- A theme file with `"accent"` values can't be imported by older versions of Comilito.
