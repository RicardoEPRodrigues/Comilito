# 0047. Decode videos without NVIDIA's hardware decoders on Linux

- Status: Accepted
- Date: 2026-11-16

## Context

WebKitGTK plays video through GStreamer, which picks a decoder by rank. With NVIDIA's drivers and
`gstreamer1.0-plugins-bad` installed, the nvcodec plugin offers `nvvp9dec` and `nvvp8dec` at the
same rank as the software decoders (`vp9dec`, `vp8dec`), and WebKit ended up with the NVIDIA ones.

Comilito often starts a video partway in: resuming it when a scene comes back, a Library
thumbnail a few seconds in, the crossfaded loop back to 0. Tested in the app's webview with real
project videos, with `nvvp9dec`:

- a video that jumped straight to a cluster in its middle, before showing any frame, often never
  finished its seek (stuck at `readyState` 1, no error) from about 24 s in;
- after a jump, some videos showed one frame and then froze, while reporting that they were
  playing.

The video window's watchdog then restarted them from the beginning after 8 s, so switching
scenes looked slow, or the video started over. The same steps with `vp9dec` worked every time,
and faster. The AppImage never had this: it bundles only the software decoders (decision 0041).

## Options considered

- **Work around it in the streamer** (load the start first, seek after the first frame): fixed
  the stuck seeks but not the frozen playback.
- **Turn the NVIDIA decoders off for Comilito:** rank 0 for them in `GST_PLUGIN_FEATURE_RANK`,
  set at startup and inherited by WebKit's web process. Only our processes are affected; other
  programs keep using them.
- **A preference to choose:** possible later, if software decoding turns out too slow for
  someone (very high resolutions on a weak CPU).

## Decision

At startup on Linux, Comilito sets `GST_PLUGIN_FEATURE_RANK` so `nvvp8dec`, `nvvp9dec`,
`nvav1dec` and their stateless (`sl`) variants are never picked. A value the user already set is
kept after ours, so it can turn them back on.

## Consequences

- Videos decode on the CPU (libvpx / aom), as in the AppImage. Fine for 1080p; 4K60 costs more
  CPU.
- VA-API decoders (Intel/AMD) aren't affected; they weren't installed on the computer this was
  tested on, so it's unknown whether they have the same problem.
