# 0061. Two Flatpak manifests, and a script for releases

- Status: Accepted
- Date: 2026-11-20

## Context

Decision 0059 had one Flatpak manifest that built this folder from generated sources
(`cargo-sources.json`, `node-sources.json`), meant to be edited into Flathub's form (the
release's git tag) when submitting. That made every local build depend on sources regenerated
after each lock-file change, and left the Flathub form to be redone by hand at each release.

A release also touches many places: the version in `package.json` and its lock file,
`Cargo.toml` and `Cargo.lock`, `tauri.conf.json`, a `<release>` in the metainfo, the
CHANGELOG's sections, a tag, the Flathub manifest's tag and commit, and the packages: the
AppImage, built on Linux, and the MSI, built on Windows. Done by hand, one is easily missed.
The two packages can't be built on the same computer, so a release spans two runs.

## Options considered

- **One manifest, rewritten for each use** (by a script, into a temporary file): no duplication,
  but the Flathub manifest wouldn't exist in the repository to review or copy.
- **Two manifests**: the local one builds this folder with network access during the build (only
  flatpak-builder allows it; Flathub doesn't), so it never needs the generated sources; the
  Flathub one builds the tag offline. Their permissions and build commands are duplicated and
  must be kept the same by hand.
- **Releases by hand, following a checklist**, or **a script** that does every step and stops
  before pushing.

## Decision

- **Two manifests** in `flatpak/`: `me.colorcrow.comilito.yml` for Flathub (git tag and commit,
  `cargo-sources.json`, `node-sources.json`, offline) and `me.colorcrow.comilito.local.yml` for
  local builds (this folder, network during the build). The Flathub one keeps Flathub's file
  name, the app ID.
- **`npm run flatpak`** (`scripts/flatpak.mjs`) builds the local manifest with Flathub's
  flatpak-builder (`org.flatpak.Builder`), installs missing runtimes from Flathub, and exports a
  single `.flatpak` file next to the AppImage. `--install` installs it, `--flathub` builds the
  Flathub manifest, `sources` regenerates the source lists with flatpak-builder-tools.
  **`npm run package`** builds the AppImage and then this Flatpak on Linux.
- **`npm run release -- vX.Y.Z`** (`scripts/release.mjs`) checks first (no uncommitted changes to
  tracked files, a newer version, the Flatpak files committed), then sets the version
  everywhere, adds the metainfo `<release>` and the CHANGELOG section, and builds this
  computer's package (the AppImage on Linux, the MSI on Windows). Only if that builds does it
  commit ("Release vX.Y.Z") and tag; a failed build undoes the version changes. On Linux it then
  regenerates the sources and points the Flathub manifest at the tag and its commit, in a second
  commit. It never pushes. `--dry-run` lists the steps.
- **An existing tag means a released version.** Tags never move once pushed, so the script never
  re-tags. If the working copy is the tagged one (apart from the Flathub manifest and sources,
  updated after tagging), it builds this computer's package, and on Linux points the Flathub
  manifest at the tag if it doesn't yet. That's how the other OS gets its package: the same
  command, run there. Otherwise it stops and says to check the tag out. Refusing would have
  needed a separate command for the second package; replacing the tag is only safe before it's
  pushed.
- The commit hash is written quoted in the manifest: YAML would read a hash of only digits, or
  one like `1e5…`, as a number.

## Consequences

- Local builds work right after a lock-file change; only releases (and `--flathub` builds) need
  the generated sources.
- The two manifests can drift apart; changes to permissions or build commands go in both.
- A release is one command per OS plus a push, and the Flathub manifest in the repository always
  names the latest release. Running it again for a released version is harmless: it rebuilds the
  package.
- On Windows, `npm run package` (and so the release) builds only the MSI, not the NSIS installer
  tauri.conf.json's `"all"` would add. That path hasn't been run yet.
- The Flathub manifest is written by hand (decision 0059's AI policy); the script only changes
  its `tag` and `commit` lines.
- The version lives in three files (`package.json`, `Cargo.toml`, `tauri.conf.json`); the script
  keeps them equal.
