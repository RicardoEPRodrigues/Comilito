# Flatpak

> **Not published for now** ([decision 0064](decisions/0064-drop-flatpak-for-now.md)). Flathub
> refused Comilito, so Linux gets the AppImage only. Everything here is kept, and still works,
> for when the Flatpak comes back: the release scripts build and upload it with `--flatpak`.

Comilito is packaged as a Flatpak with the app ID `me.colorcrow.comilito`
([decision 0059](decisions/0059-flatpak-on-flathub.md)). After Flathub refused it, each release
published a single `.flatpak` file on its GitHub page, next to the AppImage and the MSI
([decision 0063](decisions/0063-flatpak-on-github-releases.md)), until decision 0064. This page covers building it,
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

`npm run package -- --flatpak` does the same build after the AppImage (`--flatpak
--no-appimage` builds only the Flatpak). Either way the result is also a single file,
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

## Releases with the Flatpak

Releases leave the Flatpak out (decision 0064); [releasing.md](releasing.md) describes them. To
bring it back for a release, add `--flatpak` on Linux:

```bash
npm run release -- v1.1.0 --flatpak          # also builds the Flatpak, and points the Flathub manifest at the tag
npm run publish-release -- v1.1.0 --flatpak  # also uploads Comilito_x86_64.flatpak
```

With `--flatpak`, `npm run release` (on Linux) builds the Flatpak after the AppImage, then
regenerates `cargo-sources.json` and `node-sources.json`, points the Flathub manifest at the tag
and its commit, and commits that ("Point the Flathub manifest at v1.1.0"). On a tag that's
already released it does the same if the manifest doesn't name the tag yet. Those three files
may differ from the tag, since they're committed after it.

Users install the uploaded file by opening it in a software centre that takes `.flatpak` files
(GNOME Software, KDE Discover; many others only open `.flatpakref`), or with
`flatpak install --user Comilito_x86_64.flatpak`; the GNOME runtime comes from Flathub. It
doesn't update itself: they install the next version's file over it.

## Checking it

```bash
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest flatpak/me.colorcrow.comilito.yml
flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream flatpak/me.colorcrow.comilito.metainfo.xml
flatpak run --command=desktop-file-validate org.flatpak.Builder flatpak/me.colorcrow.comilito.desktop
```

The linter's rules are Flathub's; see <https://docs.flathub.org/linter>.

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
