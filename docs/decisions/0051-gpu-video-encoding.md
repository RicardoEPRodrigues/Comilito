# 0051. Convert videos on the GPU; ffmpeg from BtbN's release builds

- Status: Accepted (supersedes the ffmpeg source in [0019](0019-download-audio-from-urls.md))
- Date: 2026-11-19

## Context

Videos that may not play everywhere are converted to WebM (decision 0035) with libvpx-vp9 on the
CPU, which is slow: a 30-second 1080p60 clip took 47 s on the owner's computer. The TODO asked
for GPU encoding, and for a choice of GPU when there are two.

WebM allows VP8, VP9 and AV1. GPUs that encode one of them: NVIDIA RTX 40 and newer (AV1, NVENC),
Intel Arc and AMD RDNA 3 (AV1; VA-API on Linux, Quick Sync and AMF on Windows), older Intel
(VP9, VA-API). Older GPUs only encode H.264/HEVC, which webviews don't all play.

Measured on an RTX 5070 Ti (driver 595), 30 s of 1080p60 game footage:

| Encoder | Time | Size | SSIM vs source |
|---|---|---|---|
| libvpx-vp9, crf 32 (CPU, until now) | 47 s | 7.5 MB | 0.990 |
| av1_nvenc, cq 36 (GPU) | ≈3 s | 7.6 MB | ≈0.99 (cq 34: 0.992, cq 38: 0.988) |
| av1_vulkan, any setting | 2.5 s | 20–90 MB | 0.86–0.91 |

The AV1 result played in the video window at 60 fps with no dropped frames, like VP9.

The ffmpeg the app downloaded (yt-dlp's nightly build) failed with NVENC: its NVIDIA headers
need driver 610 or newer. BtbN's newest release build (9.0) needs the same. BtbN's 8.1 release
build works with driver 595, and NVENC keeps working with newer drivers.

## Options considered

- **The Vulkan encoders:** in every build and every vendor's driver, but on NVIDIA their quality
  was far worse at several times the size. Not used.
- **Keep yt-dlp's builds:** GPU encoding then needs driver 610 for NVIDIA users.
- **A second ffmpeg only for videos:** keeps yt-dlp's own build for downloads, but ~130 MB more.
- **BtbN's release builds for everything, pinned to a branch:** one ffmpeg; yt-dlp works with a
  standard ffmpeg (its builds only add a few patches for edge cases).

## Decision

- ffmpeg comes from BtbN's release builds (Linux and Windows), from the branch in
  `FFMPEG_BRANCH` (8.1), else the newest release if that branch stops being published. The file
  is picked from the release's checksum list, which also verifies it. `tools/ffmpeg/build.txt`
  records which build is installed; an ffmpeg without it (from yt-dlp, before this) is replaced
  by *Update*, which also replaces it when a newer build of the branch is out.
- GPU encoders (`video_encoders.rs`) are found the first time they're needed: for each candidate
  ffmpeg was built with (NVENC per NVIDIA GPU, named by `nvidia-smi`; VA-API per Intel or AMD
  render node, AV1 else VP9; Quick Sync and AMF on Windows), a tiny test encode. Only those that
  work are offered, best first. The result is kept for the session and logged.
- Preferences › Video › *Convert videos with*: Automatic (default: the first GPU that works),
  a specific GPU (which covers computers with two), or CPU only. A chosen GPU that's gone counts
  as Automatic.
- NVENC uses constant quality 36; the other GPU encoders, whose quality modes differ by driver, a
  bitrate for the picture size (about 2.5 Mbit/s for 1080p AV1, more for VP9).
- A GPU conversion that fails is redone on the CPU, with the reason in the log.

## Consequences

- Conversions are many times faster where a GPU can encode AV1, with the same quality and size.
- Converted videos are AV1 instead of VP9 there. Both play everywhere; AV1 decoding costs a
  little more CPU.
- VA-API, Quick Sync and AMF are untested (no such GPU was at hand); a failure falls back to the
  CPU, but their quality settings may need tuning.
- The pinned branch has to be raised by hand once NVIDIA drivers have caught up with the newer
  builds' headers.
- This is about encoding. Video decoding stays on the CPU (decision 0047).
