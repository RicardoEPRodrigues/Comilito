# Build environment for `npm run package` on Linux (decision 0046).
#
# The AppImage only runs on systems whose glibc is at least as new as the one it was built
# against, and glibc can't be bundled. Building on Ubuntu 22.04 (glibc 2.35) makes an AppImage
# that runs on 22.04 and everything newer, whatever distro the developer's computer runs.
# Raise this only when dropping support for older distros.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Tauri's Linux prerequisites (as in the README), plus the GStreamer plugins that
# scripts/tauri.mjs copies into the AppImage (decision 0041).
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl wget file xz-utils patchelf \
      libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev \
      librsvg2-dev libasound2-dev \
      gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad \
      gstreamer1.0-gl gstreamer1.0-alsa gstreamer1.0-pulseaudio gstreamer1.0-pipewire \
    && rm -rf /var/lib/apt/lists/*

# Node from nodejs.org: Ubuntu 22.04's own is too old (12.x).
ARG NODE_VERSION=24.13.1
RUN curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
      | tar -xJ -C /usr/local --strip-components=1

# Rust via rustup, the same way as on a developer's computer.
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable

# The AppImage tools (linuxdeploy) are AppImages themselves and normally need FUSE, which a
# container doesn't have; this makes them unpack themselves and run instead.
ENV APPIMAGE_EXTRACT_AND_RUN=1

COPY build.sh /usr/local/bin/comilito-build
RUN chmod +x /usr/local/bin/comilito-build
