# 0060. A website in `website/`, published to GitHub Pages by a workflow

- Status: Accepted
- Date: 2026-11-20

## Context

Comilito needs a website: what it does, where to download it, and the extra themes
(`extras/themes/`) for people to download. It should live on `main`, so a change to the app (a
new screenshot, a new theme) can update the site in the same commit.

GitHub Pages' "deploy from a branch" mode only publishes the repository root or `docs/`. The root
is the app's, and `docs/` holds the developer documentation; mixing website files into either
would make both harder to find your way around.

## Options considered

- **Publish `docs/`:** no workflow needed, but the site's HTML, CSS, fonts and images would sit
  among the decision records, and anything taken from elsewhere (screenshot, themes) would have
  to be copied into `docs/` and committed twice.
- **A `gh-pages` branch:** keeps `main` clean, but the site is no longer updated with the app.
- **`website/` on `main`, deployed by a GitHub Actions workflow** (Pages source: "GitHub
  Actions"): the workflow can publish any folder, and can add files from the rest of the
  repository at build time. Needs the Pages source changed once in the repository settings.
- **A static site generator or Vite/Svelte for the site:** more than one page doesn't need; it
  adds a build toolchain and dependencies.

## Decision

The website is plain HTML, CSS and a little JavaScript in `website/`, with no dependencies.
`scripts/build-website.py` copies it into `build/website/` and adds what comes from elsewhere in
the repository: the screenshot, the skeletal horse, the app icon, and the extra themes, packed by
`extras/themes/build.sh`, with `themes.json` describing them (colours, fonts, woodcut, size) so
the page can draw a preview of each. `.github/workflows/website.yml` runs it on pushes to `main`
that touch those files and deploys the result with GitHub's official Pages actions.

The page uses the app's design tokens, Codex in light mode and Ironwood in dark mode, with its
fonts (UnifrakturMaguntia, Grenze Gotisch, Pirata One, Inter; SIL OFL) served from the site
rather than a font CDN, so visitors aren't tracked by a third party.

The Horde and Alliance themes are fan themes based on Blizzard trademarks and are never
published: the build script leaves them out by name (`NOT_ON_WEBSITE`). Every other theme folder
appears on the site automatically.

## Consequences

- Adding a theme to `extras/themes/` (with a row in its README's table, used as the description)
  puts it on the website on the next push.
- The theme packages and the site's copies of the screenshot aren't committed; `build/` is
  ignored.
- The Pages source must be set to "GitHub Actions" in the repository settings once.
- A new fan theme based on someone else's trademark must be added to `NOT_ON_WEBSITE`.
