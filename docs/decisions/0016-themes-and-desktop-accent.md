# 0016. Switchable themes, custom themes and the desktop accent colour

- Status: Accepted
- Date: 2026-11-13

## Context

Design exploration in `pencil/comilito.pen` produced three directions inspired by medieval
illumination (The Brazen Boar reference): **Codex** (parchment, ember), **Ironwood** (dark) and
**Hearth** (ember sidebar). The owner wants all three available, wants users to be able to
create their own, and wants the desktop's accent colour used when the OS provides one.

## Options considered

- **One fixed look:** simplest, but drops two directions the owner wants to keep.
- **Built-in themes only:** no theme editor, but no personal palettes either.
- **Themes as token files, with built-ins plus user themes:** every colour and font is a named
  token (like the CSS variables already in `app.css`). A theme is just a set of values for them.

## Decision

Themes are sets of design tokens.

- **Built-in:** Codex, Ironwood and Hearth. They're read-only, and editing one makes a copy first.
- **User themes:** stored as JSON files in the app config folder (`~/.config/comilito/themes/`),
  and can be exported or imported to share them. A theme holds colours (page, surface, sidebar,
  header, text, muted, accent, gold/rules, track, mixer), fonts (display, body) and whether it's
  light or dark.
- **Light and dark:** either follow the desktop (one theme for light, one for dark; default
  Codex / Ironwood) or always use the selected theme.
- **Accent colour:** one of *theme colour*, *desktop accent* or *custom*. The accent marks what's
  live (top scene, playing music, active effect, play button).
  - The desktop accent is read in Rust and followed live:
    - Linux: XDG desktop portal `org.freedesktop.appearance` / `accent-color`
      (GNOME 47+, KDE Plasma 6, COSMIC)
    - Windows: `UISettings` accent colour
    - macOS: `controlAccentColor`
  - If the desktop doesn't provide an accent, the theme colour is used.
  - The accent is adjusted (lightened or darkened in OKLCH) only as much as needed to keep
    text on it, and it against the page, readable.
- **Illumination toggles:** decorated capitals, woodcut watermark, ornamental rules and
  blackletter headings can each be turned off for a plainer look.

## Consequences

- `app.css` variables become the token list, and the theme loader sets them at runtime.
- The display fonts (UnifrakturMaguntia, Grenze Gotisch, Pirata One) are bundled offline under
  the OFL and credited in CREDITS.md. Blackletter is only for headings; controls stay in a plain sans.
- New dependency for reading the portal on Linux (`ashpd` or `zbus`), and Windows/macOS
  platform calls, behind one `desktop_accent()` function. Each is to be justified when added.
- Appearance moves from a single "theme" setting to its own Preferences page (screen G in the
  design file).
