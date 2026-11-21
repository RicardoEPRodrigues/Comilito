# 0064. Drop the Flatpak for now; the AppImage is Linux's package

- Status: Accepted
- Date: 2026-11-21

## Context

Flathub refused Comilito because it's developed mostly with AI
([flathub/flathub#11289](https://github.com/flathub/flathub/pull/11289)), with nothing that would
make a submission acceptable. Decision 0063 published a `.flatpak` file on each GitHub release
instead, but that file is a poor substitute: many software centres (the COSMIC Store among them)
only open `.flatpakref` files, which point at a Flatpak repository, and a bundle never updates.
A repository of our own would fix both, at the cost of hosting it and keeping a signing key.

## Options considered

- **Keep publishing the `.flatpak` file**: many users can't open it without a terminal.
- **Host a Flatpak repository** (on GitHub Pages, with a `.flatpakref`): updates and one-click
  installs, but it's the self-hosting the owner wants to avoid, plus a GPG key to keep safe.
- **Drop the Flatpak, keep the AppImage**: one Linux package to support. The Flatpak files
  stay, unused, in case Flathub changes its mind.

## Decision

Linux gets the **AppImage** only. The website offers two downloads, Linux (AppImage) and
Windows (MSI), with a note on why there's no Flatpak.

Everything Flatpak is kept and still works, behind a flag: `npm run package -- --flatpak`,
`npm run release -- vX.Y.Z --flatpak` (which also points the Flathub manifest at the tag) and
`npm run publish-release -- vX.Y.Z --flatpak`. `npm run flatpak` is unchanged. The sandbox fixes
in the app (`flatpak.rs`, `trash_bin.rs`) stay; they do nothing outside a Flatpak.

This supersedes decision 0063's Flatpak file; its GitHub releases and version-free file names
stay.

## Consequences

- Releases no longer need flatpak-builder or flatpak-builder-tools.
- The AppImage doesn't add itself to the desktop's app menu; that's for Comilito to do, or for
  tools like Gear Lever and AppImageLauncher.
- The Flatpak code paths aren't exercised by releases, so they may break unnoticed; try
  `npm run flatpak` before bringing it back.
