# 0052. A preference to decode videos on NVIDIA GPUs (Linux)

- Status: Accepted (adds an opt-out to [0047](0047-no-nvidia-video-decoding.md))
- Date: 2026-11-19

## Context

The owner asked for video (and audio) playback on the GPU when it helps. Audio mixing in kira
takes very little CPU; there's nothing to gain there.

Video: Windows (WebView2) and macOS (WKWebView) already decode on the GPU by themselves. On
Linux, WebKitGTK decodes through GStreamer, which already uses Intel and AMD decoders (VA-API)
when installed. Only NVIDIA's decoders are kept out, by decision 0047, because they froze after
jumps into a video.

Retested on a virtual display (GStreamer 1.20, driver 595, RTX 5070 Ti) with NVIDIA's VP9
decoder allowed again: 1080p60 played without dropped frames, ten jumps around the video and six
resumes partway in (closing and reopening the video window) all played on. Since 0047 the way
videos are loaded has changed (decision 0037), which may be why. The CPU saving couldn't be
measured there: without a real GPU, WebKit copies every frame back and draws in software.
GStreamer 1.20 has no NVIDIA AV1 decoder (it came in 1.22), so AV1 videos still decode on the CPU
on such systems.

## Options considered

- **Turn NVIDIA decoding back on for everyone:** the freezes may be gone, but that's one test on
  one computer, and the benefit isn't measured yet.
- **An opt-in preference:** what 0047 foresaw. People who want it can try it; nothing changes
  for anyone else.

## Decision

Preferences › Video › "Decode videos on the GPU (NVIDIA, experimental)", off by default, Linux
only (hidden in the AppImage, which carries only software decoders, decision 0041). On, the app
doesn't set 0047's `GST_PLUGIN_FEATURE_RANK` blocklist, and GStreamer picks NVIDIA's decoders.

WebKit's processes take the environment when they start, so the preference is read from
`preferences.json` at the very start (before Tauri; `early_config_dir` in lib.rs finds it as
Tauri would) and applies from the next start. Preferences says so when the switch differs from
what the running app uses (`gpu_decode` command).

## Consequences

- Trying GPU decoding is a switch and a restart, and turning it off is the same.
- To decide later: measure the CPU saving on a real desktop, and whether anything freezes, before
  making it the default.
