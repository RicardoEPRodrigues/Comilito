# 0046. Package Linux builds as an AppImage, in an Ubuntu 22.04 container

- Status: Accepted
- Date: 2026-11-16

## Context

An AppImage built on a laptop with a newer OS (glibc 2.39) crashed elsewhere with
"`GLIBC_2.39' not found". A program built against one glibc only runs where glibc is the same
or newer. glibc can't be bundled: it comes as a set with the dynamic loader, whose path is fixed
in every executable, and every system library the app loads (WebKitGTK, GTK, Mesa, the audio
libraries) links against the host's glibc. So the build machine's glibc sets the minimum for
users, and builds depended on which computer they happened to be made on.

`tauri build` also made a `.deb` and an `.rpm` on Linux. Each is tied to a family of distros and
needs testing there too.

## Options considered

- **Build on whatever computer is at hand:** simple, but the result depends on its OS.
- **`cargo-zigbuild` targeting an older glibc:** only covers the Rust code; the app still links
  against the host's WebKitGTK and GTK, built against the host's newer glibc.
- **Build in a container of an older distro:** the same result from any computer; needs Docker.
- **Flatpak:** the app runs on a fixed runtime with its own glibc and WebKitGTK, so the host's
  versions don't matter. Wanted, but a bigger job (manifest, offline Cargo and npm sources).

## Decision

On Linux Comilito is distributed as an **AppImage**, with **Flatpak** to follow later. The `.deb`
and `.rpm` are dropped (`bundle.targets` in `tauri.linux.conf.json` is only `appimage`). Windows
keeps its installers.

`npm run package` is the way to make release packages (`scripts/package.mjs`). On Linux, when
Docker is running, it builds in an Ubuntu 22.04 container (`scripts/package/Dockerfile`, glibc
2.35), so the AppImage runs on Ubuntu 22.04 and newer, and distros of the same age. The container
copies the project and builds the copy, so it never writes into the developer's folder; Cargo's
downloads, the compiled Rust and `node_modules` are kept in Docker volumes between runs. The
AppImage is copied to `src-tauri/target/release/bundle/appimage/`, as with a local build.

Without Docker it builds on the computer itself and warns which glibc the AppImage then needs.
`--docker` and `--local` force one or the other. On other OSes it runs `tauri build`.

## Consequences

- AppImages run on the same range of systems whichever computer made them.
- The first Docker build downloads the image and compiles everything (several minutes); later
  ones reuse the volumes. `docker volume rm` on the `comilito-package-*` volumes frees the space.
- Raising the oldest supported distro means changing the `FROM` line in the Dockerfile.
- No package for apt or dnf users; they use the AppImage until there's a Flatpak.
