#!/bin/sh
# Runs inside the container started by scripts/package.mjs.
#
# /src is the project, mounted read-only; /out is where the AppImage goes on the host.
# The project is copied first so the build never writes into the developer's folder: the
# container runs as root and would leave root-owned files there, and its node_modules and
# compiled Rust (linked against an older glibc) must not mix with the host's.
set -eu

mkdir -p /work
tar -C /src --exclude=./node_modules --exclude=./dist --exclude=./src-tauri/target \
    --exclude=./.git -cf - . | tar -C /work -xf -
cd /work

# node_modules and the Rust target folder live in Docker volumes (mounted by package.mjs) so
# later runs only rebuild what changed.
npm ci
npm run tauri build -- --bundles appimage

rm -f /out/*.AppImage
cp "$CARGO_TARGET_DIR"/release/bundle/appimage/*.AppImage /out/
chown "$HOST_UID:$HOST_GID" /out/*.AppImage
