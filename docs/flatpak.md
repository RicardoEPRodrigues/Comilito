# Flatpak

Comilito is packaged as a Flatpak with the app ID `me.colorcrow.comilito`
([decision 0059](decisions/0059-flatpak-on-flathub.md)). Flathub refused it, so each release
publishes a single `.flatpak` file on its GitHub page, next to the AppImage and the MSI
([decision 0063](decisions/0063-flatpak-on-github-releases.md)). This page covers building it,
trying it, releasing a version and publishing it
([decision 0061](decisions/0061-flatpak-builds-and-releases.md)).

## The pieces

| File | What it is |
|---|---|
| `flatpak/me.colorcrow.comilito.yml` | The **Flathub manifest**: the runtime to run on, the sandbox's permissions (`finish-args`), and how to build Comilito from a release's git tag, offline. Unused since Flathub refused Comilito (decision 0063), kept in case it's tried again |
| `flatpak/me.colorcrow.comilito.local.yml` | The **local manifest**: the same, but builds this folder as it is, downloading crates and npm packages during the build |
| `flatpak/me.colorcrow.comilito.desktop` | The launcher entry (name, icon, categories) |
| `flatpak/me.colorcrow.comilito.metainfo.xml` | The **metainfo** (AppStream): the description, screenshot and releases software centres show |
| `flatpak/cargo-sources.json`, `flatpak/node-sources.json` | Every crate and npm package, generated from the lock files, so the Flathub build runs offline |

The two manifests must stay the same apart from where the sources come from: change the
permissions or the build commands in both.

Some Flatpak words:

- **Runtime**: the shared base the app runs on. Comilito uses GNOME's (`org.gnome.Platform`),
  which carries WebKitGTK and GStreamer. Users get it once, shared by every app that uses it.
- **SDK**: the runtime plus compilers and headers, used to build. Rust and Node come as **SDK
  extensions**.
- **`flatpak-builder`**: reads the manifest and builds in a sandbox, without network access.
- **finish-args**: what the app may reach outside its sandbox.
- Inside the sandbox the app's own folders are under `~/.var/app/me.colorcrow.comilito/`
  (`config/`, `data/`, `cache/`), apart from the AppImage's.

## One-time setup

```bash
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder org.gnome.Sdk//51 \
  org.freedesktop.Sdk.Extension.rust-stable//26.08 org.freedesktop.Sdk.Extension.node24//26.08
```

`org.flatpak.Builder` is flatpak-builder packaged by Flathub, with its linter; it's what
Flathub builds with. The SDK extensions' branch (`26.08`) is the freedesktop base GNOME 51 is
built on; raise both together.

The source generators come from [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools),
in a Python virtual environment of their own:

```bash
git clone https://github.com/flatpak/flatpak-builder-tools.git ~/src/flatpak-builder-tools
python3 -m venv ~/src/flatpak-builder-tools/venv
~/src/flatpak-builder-tools/venv/bin/pip install aiohttp tomlkit ~/src/flatpak-builder-tools/node
```

## Building and running

```bash
npm run flatpak                  # builds this folder (the local manifest)
npm run flatpak -- --install     # …and installs it for your user
flatpak run me.colorcrow.comilito
```

`npm run package` does the same build after the AppImage (`--no-flatpak` skips it,
`--no-appimage` builds only the Flatpak). Either way the result is also a single file,
`src-tauri/target/release/bundle/flatpak/Comilito_<version>_x86_64.flatpak`, which installs with
`flatpak install Comilito_….flatpak` (the runtime comes from Flathub). The script
(`scripts/flatpak.mjs`) uses Flathub's flatpak-builder, and installs the runtime, the SDK and its
extensions the first time.

`npm run flatpak -- --flathub` builds the Flathub manifest instead: the tagged release, offline,
exactly as Flathub will. It needs the tag pushed to GitHub.

The first build compiles everything (several minutes); later ones reuse what didn't change.
`flatpak/build`, `flatpak/repo` and `flatpak/.flatpak-builder` are ignored by git.

- Logs: `~/.var/app/me.colorcrow.comilito/data/me.colorcrow.comilito/logs/`.
- A shell inside the sandbox, to look around: `flatpak run --command=sh me.colorcrow.comilito`.
- Uninstall: `npm run flatpak -- uninstall`. Its data (settings, backups, downloaded tools in
  `~/.var/app/me.colorcrow.comilito/`) stays for the next install; add `--delete-data` to remove
  it too. Projects are never touched.

## Generating the sources

The Flathub manifest builds offline from `cargo-sources.json` and `node-sources.json`. `npm run
release` regenerates them; to do it by hand (say, to try `--flathub` after a lock file changed):

```bash
npm run flatpak -- sources
```

It runs flatpak-builder-tools from `~/src/flatpak-builder-tools` (set `FLATPAK_BUILDER_TOOLS` if
they're elsewhere).

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
2. builds this computer's packages: the **AppImage** and the **Flatpak** on Linux (as
   `npm run package`, the AppImage in Docker when it runs), the **MSI** on Windows. If the build
   fails, the version changes are undone and nothing is committed;
3. commits ("Release v1.1.0") and tags it `v1.1.0`;
4. on Linux, regenerates the sources, points the Flathub manifest at the tag and its commit,
   and commits that ("Point the Flathub manifest at v1.1.0").

If the tag already exists, the version is released: a tag never moves, so it only builds this
computer's package for it (and on Linux points the Flathub manifest at it, if it doesn't yet).
The working copy must be the tagged one, apart from the Flathub manifest and sources; otherwise
it says to `git checkout v1.1.0` first. That's how both packages get made:

- release on Linux (AppImage, Flathub manifest), pull on Windows and run the same command (MSI);
- or release on Windows (MSI), pull on Linux and run the same command (AppImage, Flathub manifest).

It never pushes. Check the commits, push (`git push origin HEAD v1.1.0`), then publish.

## Checking it

```bash
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/me.colorcrow.comilito.yml
flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream flatpak/me.colorcrow.comilito.metainfo.xml
flatpak run --command=desktop-file-validate org.flatpak.Builder flatpak/me.colorcrow.comilito.desktop
```

The linter's rules are Flathub's; see <https://docs.flathub.org/linter>.

## Publishing a release

```bash
npm run publish-release -- v1.1.0 --dry-run   # which files it would upload
npm run publish-release -- v1.1.0
```

`scripts/publish-release.mjs` puts this computer's packages for the version on its GitHub
release, with the [GitHub CLI](https://cli.github.com) (`gh`, logged in with `gh auth login`).
Run it on Linux (AppImage, Flatpak) and on Windows (MSI); the first run creates the release,
with the version's CHANGELOG section as its notes, and the other adds its files to it.

- The tag must be pushed first, and be the same commit on GitHub as here; otherwise it stops.
- It uploads the files without their version (`Comilito_amd64.AppImage`,
  `Comilito_x86_64.flatpak`, `Comilito_x64_en-US.msi`), so the website's buttons can link to
  `…/releases/latest/download/<name>`, the newest release's file. Don't rename them.
- Running it again replaces the files of the same name (a rebuilt package).
- Until both OSes have published, the newest release lacks one OS's file and its website
  button gives "Not Found"; publish both close together.

Users install the Flatpak by opening the file in their software centre, or with
`flatpak install --user Comilito_x86_64.flatpak`; the GNOME runtime comes from Flathub. It
doesn't update itself: they install the next version's file over it.

## Flathub (refused)

Flathub refused Comilito: the repository was too new, and much of it was written with AI
([decision 0063](decisions/0063-flatpak-on-github-releases.md)). The Flathub manifest and its
sources are kept, and `npm run release` still points the manifest at each tag, in case it's tried
again. Flathub's [requirements](https://docs.flathub.org/docs/for-app-authors/requirements) and
[submission guide](https://docs.flathub.org/docs/for-app-authors/submission) say what that
takes: the manifest written by hand (no AI-generated or AI-assisted content), the AI-written
code disclosed, the pull request opened and answered by hand, and the app ID verified with a
token at `https://colorcrow.me/.well-known/org.flathub.VerifiedApps.txt` (or a DNS TXT record on
`_flathub.colorcrow.me`).
