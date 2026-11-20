# 0059. Distribute a Flatpak on Flathub, as `me.colorcrow.comilito`

- Status: Accepted
- Date: 2026-11-20

## Context

Decision 0046 made the AppImage the Linux package, with a Flatpak to follow. A Flatpak runs on
a fixed runtime (its own glibc, WebKitGTK and GStreamer), so it works the same on every distro,
and Flathub puts it in the software centres most Linux users install apps from (GNOME Software,
KDE Discover, the COSMIC Store).

A Flatpak needs an app ID, a reverse-DNS name. Desktops use it to match a window to its
launcher and icon: on Wayland a window's app ID must equal the `.desktop` file's name. Tauri
uses its `identifier` as that app ID, and also names the app's folders after it
(`~/.local/share/<identifier>` and so on).

Flathub's requirements (read on 2026-11-20) shape the rest:

- Open-source apps must be built from source, with every dependency listed in the manifest, and
  without network access during the build.
- The app ID's domain must be under the developer's control, which Flathub verifies with a
  token at `https://<domain>/.well-known/org.flathub.VerifiedApps.txt` (or a DNS record).
- Generative AI: the manifest must not contain AI-generated or AI-assisted content, AI must not
  open the submission or write its messages, and AI-generated code in the app must be disclosed
  (which parts, roughly how much). Reviewers decide on disclosed AI code and may reject it.
  Comilito is mostly written with Claude (see the README).

## Options considered

- **Self-hosted Flatpak** (a `.flatpak` file on the releases page, or our own repository): no
  review and no AI rule, but users must find and install it by hand, and updates need our own
  repository. Not wanted.
- **Flathub**: where users look for apps, with updates for free. Needs the manifest written by
  hand and a review that may refuse the app over its AI-written code.
- **App ID `io.github.ricardoeprodrigues.Comilito`**: verifiable without a domain, but tied to
  where the code is hosted. **`me.colorcrow.comilito`**: the author's own domain, independent
  of the code host.

## Decision

Comilito goes to **Flathub** as **`me.colorcrow.comilito`**, which is also Tauri's identifier
from now on.

- **Identifier.** `identifier` in tauri.conf.json and `app_id::APP_ID` in the code are
  `me.colorcrow.comilito` (a test keeps them equal), on every OS.
- **Runtime.** `org.gnome.Platform` 51: WebKitGTK 4.1, GStreamer 1.28 with every plugin the
  video window uses. None of the AppImage's plugin picking (decision 0041) applies.
- **Build.** The manifest (`flatpak/me.colorcrow.comilito.yml`) builds the frontend with
  `npm ci` and `vite build`, then the app with plain `cargo build --features
  tauri/custom-protocol` (what `tauri build` does, without the Tauri CLI's native binary). The
  crates and npm packages come from `cargo-sources.json` and `node-sources.json`, generated from
  the lock files with flatpak-builder-tools. The sample audio goes to `/app/lib/Comilito/`, where
  Tauri looks for resources next to `/app/bin/comilito`.
- **Permissions.** Wayland (X11 as fallback), the GPU (`dri`), PulseAudio, the network
  (downloads), the MPRIS name for media controls, `org.freedesktop.FileManager1` so "Show in
  folder" selects the file, and the files: the home folder plus `/media`, `/run/media` and
  `/mnt`, where other disks and removable drives are mounted. Projects are folders that
  Comilito writes into, with relative paths inside, anywhere the user chooses (decision 0042).
  The file chooser goes through the portal inside the sandbox; for a folder the app can reach
  it returns the real path, but for any other it lends a read-only copy under
  `/run/user/<uid>/doc/`, where saving fails. Comilito refuses to open or create projects
  there, saying why (`flatpak.rs`). Older portals (before version 5, e.g. Ubuntu 22.04's) can't
  tell a sandboxed app the real path behind such a copy, so translating it back isn't possible.
- **Rendering on NVIDIA.** With the runtime's WebKitGTK and NVIDIA's driver, WebKit's DMA-BUF
  renderer can't allocate its buffers ("Failed to create GBM buffer … Invalid argument") and
  the window stays white, on X11 and Wayland alike; skipping only GBM
  (`WEBKIT_DMABUF_RENDERER_DISABLE_GBM`) doesn't help. Inside the Flatpak on NVIDIA, Comilito
  sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` at startup, before WebKit starts. Intel and AMD keep
  the DMA-BUF renderer, and so do other builds, where the system's WebKitGTK works on NVIDIA.
- **Trash.** Inside the sandbox, `$XDG_DATA_HOME` is private, so the `trash` crate would move
  files to a trash no file manager shows. In a Flatpak, Comilito asks the trash portal
  (`org.freedesktop.portal.Trash`) instead (`trash_bin.rs`).
- **Metadata.** `flatpak/` also holds the `.desktop` file and the AppStream metainfo
  (description, screenshot, releases), installed under the app ID.

## Consequences

- Linux users get Comilito from their software centre, with updates, on any distro. The
  AppImage stays for those who prefer it.
- The sandbox keeps Comilito's data in `~/.var/app/me.colorcrow.comilito/`, apart from the
  AppImage's. Moving from one to the other starts with fresh settings and backups (projects are
  shared, being in the home folder).
- Every Cargo.lock or package-lock.json change needs `cargo-sources.json` and
  `node-sources.json` regenerated before a Flathub update.
- `colorcrow.me` must serve the verification token over HTTPS, and keep doing so. The ID can
  hardly change once published.
- Flathub may still refuse Comilito over its AI-written code; then a self-hosted Flatpak is the
  fallback.
