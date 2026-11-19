# 0053. Carry NVIDIA's GStreamer decoders in the AppImage

- Status: Accepted (extends [0041](0041-gstreamer-in-the-appimage.md) and
  [0052](0052-gpu-video-decoding-preference.md))
- Date: 2026-11-19

## Context

Decision 0052 added a switch to decode videos on NVIDIA GPUs, but hid it in the AppImage: the
AppImage carries its own GStreamer with a chosen list of plugins (0041), and NVIDIA's decoders
(`nvcodec`, in plugins-bad) weren't on it. The owner runs the AppImage and wants the switch there.

## Options considered

- **Use the system's `nvcodec` plugin from the AppImage:** it's built against the system's
  GStreamer, which may not match the bundled one (the reason 0041 rejected the system's plugins).
- **Bundle `nvcodec` with the other plugins:** it links only to GStreamer libraries the AppImage
  already carries, and loads CUDA and NVDEC (`libcuda.so.1`, `libnvcuvid.so.1`) from the
  system's NVIDIA driver when GStreamer scans it. It carries no codec of its own (the decoding
  is the driver's), so it adds none of the patent concerns 0041 avoided. About 350 KB.

## Decision

`nvcodec` joins the optional plugins in `scripts/tauri.mjs`. The switch shows on Linux whether
or not Comilito runs as an AppImage. With it off, the startup rank override of 0047 keeps the
NVIDIA decoders unused, as before.

## Consequences

- On computers without NVIDIA's driver, the plugin finds no CUDA and registers no decoders;
  nothing else changes.
- The NVIDIA decoders come from the GStreamer the AppImage is built with (Ubuntu 22.04's 1.20,
  decision 0046): VP8 and VP9 only, no AV1, whatever the system's GStreamer has.
