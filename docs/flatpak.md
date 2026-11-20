# Flatpak

Comilito is packaged as a Flatpak for Flathub with the app ID `me.colorcrow.comilito`
([decision 0059](decisions/0059-flatpak-on-flathub.md)). This page covers building it, trying
it, releasing a version and getting it onto Flathub
([decision 0061](decisions/0061-flatpak-builds-and-releases.md)).

## The pieces

| File | What it is |
|---|---|
| `flatpak/me.colorcrow.comilito.yml` | The **Flathub manifest**: the runtime to run on, the sandbox's permissions (`finish-args`), and how to build Comilito from a release's git tag, offline |
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
- Uninstall, with its data: `flatpak uninstall --user --delete-data me.colorcrow.comilito`.

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

`scripts/release.mjs` refuses to start if tracked files have uncommitted changes, the tag exists
or the version isn't newer. Then it:

1. sets the version in `package.json` and `package-lock.json`, `src-tauri/Cargo.toml` and
   `Cargo.lock`, and `tauri.conf.json`; adds a `<release>` to the metainfo; and turns what's
   under **Unreleased** in CHANGELOG.md into the version's section;
2. commits that ("Release v1.1.0") and tags it `v1.1.0`;
3. regenerates the sources, points the Flathub manifest at the tag and its commit, and commits
   that ("Point the Flathub manifest at v1.1.0").

It never pushes. Check the two commits, push (`git push origin HEAD v1.1.0`), then copy the
Flathub manifest and the two source files to the Flathub repository.

## Checking it

```bash
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/me.colorcrow.comilito.yml
flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream flatpak/me.colorcrow.comilito.metainfo.xml
flatpak run --command=desktop-file-validate org.flatpak.Builder flatpak/me.colorcrow.comilito.desktop
```

The linter's rules are Flathub's; see <https://docs.flathub.org/linter>.

## Publishing on Flathub

Read Flathub's [requirements](https://docs.flathub.org/docs/for-app-authors/requirements) and
[submission guide](https://docs.flathub.org/docs/for-app-authors/submission) first. In short:

1. **Make the repository public and the website live**, so the metainfo's links and
   screenshot load (the homepage is `https://comilito.colorcrow.me`).
2. **Release a version** with `npm run release -- v1.0.0`, and push it.
3. **Write the manifest for Flathub.** It builds from the tag, not from this folder.
   Flathub doesn't accept manifests with AI-generated or AI-assisted content, and
   `flatpak/me.colorcrow.comilito.yml` was drafted with Claude, so the submitted manifest must
   be written by hand; the one in this repository is the reference that's known to build.
4. **Submit**: fork [flathub/flathub](https://github.com/flathub/flathub), branch from
   `new-pr`, add the manifest and the two generated source files, and open a pull request
   against `new-pr`. Disclose the AI-written code in it, as Flathub requires. Opening the pull
   request and answering the reviewers must be done by hand, not by an AI tool.
5. **Verify the app ID**: once accepted, Flathub's website gives a token to publish at
   `https://colorcrow.me/.well-known/org.flathub.VerifiedApps.txt` (HTTPS required). It's the
   domain the ID names, `colorcrow.me`, not the `comilito.` subdomain the website is on; a DNS
   TXT record on `_flathub.colorcrow.me` works too.
6. **Updates**: Flathub creates `flathub/me.colorcrow.comilito`; each release is a pull request
   there that updates the tag, the commit and the generated sources, as `npm run release` left
   them here.
