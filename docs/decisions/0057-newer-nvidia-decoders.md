# 0057. Use nvcodec's newer decoders on GStreamer 1.20

- Status: Accepted (refines [0054](0054-gpu-video-decoding-by-default.md))
- Date: 2026-11-20

## Context

With GPU decoding on (decision 0054) and videos played from their files (decision 0056), the
owner still saw scene switches lag on their desktop (RTX 5070 Ti, driver 595), and videos
sometimes stop playing altogether: what decision 0047 saw. It never showed on a virtual display.

GStreamer 1.20's nvcodec plugin has two decoders per codec:

- `nvvp9dec`, `nvvp8dec`: the older ones, built on NVIDIA's own parser, rank primary (256).
- `nvvp9sldec`, `nvvp8sldec`: newer "stateless" ones, built on GStreamer's common decoder base
  classes (the same flushing and seeking code as its other decoders), rank secondary (128).

GStreamer picks the highest rank, so the older ones were always used. In GStreamer 1.22 the
newer ones took over the plain names and the older ones were removed: upstream's own verdict.

## Options considered

- **Turn NVIDIA decoding off again:** gives up the GPU on every NVIDIA computer.
- **Prefer the newer decoders where both exist:** on GStreamer before 1.22, rank the older ones
  out (0) and the newer ones just above the software decoders (257). Tested headless with six
  scene switches resuming VP9 videos partway in: `nvvp9sldec` was used for all, each moving
  again about as fast as with the software decoder. The desktop test is the owner's.

## Decision

`nvidia_decoder_ranks` in `lib.rs` sets `GST_PLUGIN_FEATURE_RANK` at startup: all NVIDIA
decoders out with GPU decoding off; with it on and GStreamer older than 1.22, the older decoders
out and the newer in; with 1.22 or newer, nothing. The version is the GStreamer WebKit will load
(`gst_version` through `dlopen`, which inside the AppImage finds the bundled one).

## Consequences

- If the newer decoders still lag on the owner's desktop, GPU decoding goes back to off by
  default: nothing else is left to choose from on GStreamer 1.20.
- On 1.22 and newer, AV1 also has an NVIDIA decoder; those systems haven't been tested.
