# 0063. Publish the Flatpak ourselves, on GitHub releases

- Status: The Flatpak superseded by [0064](0064-drop-flatpak-for-now.md) (GitHub releases and their file names still apply)
- Date: 2026-11-20

## Context

Decision 0059 chose Flathub for the Flatpak. Flathub refused the submission: the repository
was too new (they expect a project to be a month or two old), and, the reason that made the
refusal final, much of Comilito was written with AI.

The AppImage and the MSI were meant for the GitHub releases page, but nothing put them there:
the website's buttons only opened the newest release, and its Flatpak button said "Coming soon".

## Options considered

- **Our own Flatpak repository** (a `.flatpakrepo` served from the website), so Flatpak updates
  it like a Flathub app: needs the repository signed with a GPG key, hosted and kept in step
  with each release. The owner doesn't want to self-host.
- **A single `.flatpak` file on each GitHub release** (`flatpak build-bundle`, which
  `npm run flatpak` already makes): no hosting; software centres install it, and fetch the GNOME
  runtime from Flathub, which the file names. It doesn't update itself: users install the next
  file over it.
- **Uploading by hand** on GitHub's website, or **a script with the GitHub CLI** (`gh release
  create` / `gh release upload`).

## Decision

Each release's GitHub page carries the packages: the AppImage and the Flatpak (built on Linux)
and the MSI (built on Windows). `npm run release` builds the Flatpak along with the AppImage;
`npm run publish-release -- vX.Y.Z` (`scripts/publish-release.mjs`) uploads this computer's
packages with `gh`, creating the release (notes from the version's CHANGELOG section) or adding
to it.

The files are uploaded without their version: `Comilito_amd64.AppImage`,
`Comilito_x86_64.flatpak`, `Comilito_x64_en-US.msi`. GitHub serves the newest release's files
at `https://github.com/RicardoEPRodrigues/Comilito/releases/latest/download/<name>`, so the
website's buttons download them directly and never need editing.

The script refuses to run unless the tag is on GitHub and is the same commit as here:
`gh release create` would otherwise make the tag itself, from the default branch.

## Consequences

- The Flatpak installs from a file and doesn't update itself; a later version could check for
  updates (TODO.md, v1.1.0).
- Between publishing on one OS and the other, the newest release lacks the other OS's file, so
  its button gives GitHub's "Not Found". Publish both close together.
- The upload names must stay as they are, or the website's links break.
- The Flathub manifest (`flatpak/me.colorcrow.comilito.yml`) and its generated sources stay in
  the repository for now; nothing uses them unless Flathub is tried again.
- Needs the GitHub CLI, logged in (`gh auth login`), on each computer that publishes.
