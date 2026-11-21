# Releasing

A release is a version number, a git tag and a page on GitHub's releases with the packages:
the **AppImage** (built on Linux) and the **MSI** installer (built on Windows). The website's
buttons download the newest release's files. Two scripts do the work
([decisions 0061](decisions/0061-flatpak-builds-and-releases.md),
[0063](decisions/0063-flatpak-on-github-releases.md), [0064](decisions/0064-drop-flatpak-for-now.md)):

1. `npm run release -- v1.1.0`: sets the version, builds this computer's package, commits and
   tags. Run it on one OS, then on the other for the same tag.
2. `npm run publish-release -- v1.1.0`: after pushing, uploads this computer's package to the
   GitHub release. Run it on both OSes.

The Flatpak isn't part of releases for now; `--flatpak` on both commands brings it back
([flatpak.md](flatpak.md#releases-with-the-flatpak)).

## Releasing a version

```bash
npm run release -- v1.1.0 --dry-run   # what it would do
npm run release -- v1.1.0
```

`scripts/release.mjs` refuses to start if tracked files have uncommitted changes. For a new
version (no tag `v1.1.0` yet, and newer than the current one) it:

1. sets the version in `package.json` and `package-lock.json`, `src-tauri/Cargo.toml` and
   `Cargo.lock`, and `tauri.conf.json`; adds a `<release>` to the metainfo; and turns what's
   under **Unreleased** in CHANGELOG.md into the version's section;
2. builds this computer's package: the **AppImage** on Linux (as `npm run package`, in Docker
   when it runs), the **MSI** on Windows. If the build fails, the version changes are undone and
   nothing is committed;
3. commits ("Release v1.1.0") and tags it `v1.1.0`.

If the tag already exists, the version is released: a tag never moves, so it only builds this
computer's package for it. The working copy must be the tagged one; otherwise it says to
`git checkout v1.1.0` first. That's how both packages get made:

- release on Linux (AppImage), pull on Windows and run the same command (MSI);
- or release on Windows (MSI), pull on Linux and run the same command (AppImage).

It never pushes. Check the commits, push (`git push origin HEAD v1.1.0`), then publish.

### Releasing a version again (`--force`)

```bash
npm run release -- v1.1.0 --force
```

For a release that was withdrawn (its tag and GitHub release deleted) or needs redoing: the
version is released as if it were new, even though it isn't newer than the current one.

- A local tag `v1.1.0` is deleted, and made again on the new release commit.
- The version's metainfo `<release>` and CHANGELOG section, if they exist, get today's date,
  and what's under **Unreleased** moves into that section, each entry under its heading
  (### Added, ### Changed…), before the ones already there.
- With nothing new to commit, the tag goes on the current commit.
- A tag still on GitHub isn't touched: it warns, and then either delete it
  (`git push origin :refs/tags/v1.1.0`) or replace it (`git push --force origin v1.1.0`). Delete
  the GitHub release on it too, then publish again.

## Publishing a release

```bash
npm run publish-release -- v1.1.0 --dry-run   # which files it would upload
npm run publish-release -- v1.1.0
```

`scripts/publish-release.mjs` puts this computer's package for the version on its GitHub
release, with the [GitHub CLI](https://cli.github.com) (`gh`, logged in with `gh auth login`).
Run it on Linux (AppImage) and on Windows (MSI); the first run creates the release, with the
version's CHANGELOG section as its notes, and the other adds its file to it.

- The tag must be pushed first, and be the same commit on GitHub as here; otherwise it stops
  (`gh` would otherwise create the tag itself, from the default branch).
- It uploads the files without their version (`Comilito_amd64.AppImage`,
  `Comilito_x64_en-US.msi`), so the website's buttons can link to
  `…/releases/latest/download/<name>`, the newest release's file. Don't rename them.
- Running it again replaces the files of the same name (a rebuilt package).
- Until both OSes have published, the newest release lacks one OS's file and its website
  button gives "Not Found"; publish both close together.
