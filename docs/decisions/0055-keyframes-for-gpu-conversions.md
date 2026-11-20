# 0055. A keyframe every 60 frames in GPU conversions

- Status: Accepted (adds to [0051](0051-gpu-video-encoding.md))
- Date: 2026-11-20

## Context

With GPU decoding on (decision 0054), the owner saw scene switches lag again, but only with
videos converted on the GPU. Those are AV1 (decision 0051). On Linux with GStreamer 1.20 (Ubuntu
22.04 and the AppImage) there's no hardware AV1 decoder for NVIDIA, so AV1 decodes on the CPU
with libaom's `av1dec`, single-threaded: about 100 fps for real 1080p60 footage, against several
hundred for VP9.

A scene that comes back resumes its video partway in. The decoder starts at the keyframe before
that point and decodes forward to it. NVENC puts a keyframe every 250 frames by default (4.2 s
at 60 fps), so a jump could mean up to 250 frames, about 2.5 s, before the picture moved. The
CPU encoder (libvpx) uses 128.

## Options considered

- **More keyframes in GPU conversions (`-g 60`):** a jump decodes at most 60 frames, under a
  second even with libaom. Measured on 30 s of real 1080p60: 19.7 MB with the default, 20.1 MB
  with 60 (+2%), same quality setting.
- **Convert to VP9 on the CPU on Linux:** NVIDIA's VP9 decoder would play it, but conversions
  would be many times slower again, which is what 0051 set out to fix.
- **A faster AV1 decoder** (dav1d): not in GStreamer 1.20's plugin sets; it's a Rust plugin or
  comes through libav, which decision 0041 keeps out of the AppImage.

## Decision

Every GPU encoder gets `-g 60` (`video_encoders.rs`). The interval is in frames on purpose:
the time a jump costs depends on how many frames the decoder works through.

## Consequences

- GPU conversions are about 2% larger.
- Videos already converted on the GPU keep their old keyframes; converting them again (from the
  original) gives the new ones.
- AV1 still decodes on the CPU on such systems: about one core for 1080p60, and 4K60 AV1 may not
  keep up. GStreamer 1.22 and newer have NVIDIA's AV1 decoder.
