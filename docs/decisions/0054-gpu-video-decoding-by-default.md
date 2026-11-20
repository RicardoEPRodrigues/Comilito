# 0054. Decode videos on NVIDIA GPUs by default (Linux)

- Status: Accepted (supersedes [0047](0047-no-nvidia-video-decoding.md); changes the default of
  [0052](0052-gpu-video-decoding-preference.md))
- Date: 2026-11-20

## Context

Decision 0047 kept NVIDIA's GStreamer decoders out because videos froze after jumps. Decision
0052 let people turn them back on with a switch, off by default; 0053 brought it to the AppImage.
The freezes didn't come back in testing: ten jumps and six resumes on a virtual display, and the
owner's own use of the switch on their desktop (RTX 5070 Ti, driver 595). The owner asked for it
on by default.

## Options considered

- **Keep it off by default:** safe, but everyone with an NVIDIA GPU decodes on the CPU, and few
  would find the switch.
- **On by default, the switch to turn it off:** GPU decoding where it works; if the freezes come
  back for someone, the switch (and a restart) is the way out, and the preference's help says so.

## Decision

`video.gpuDecode` defaults to `true`. The switch stays in Preferences › Video, without
"experimental". With it off, the startup rank override of 0047 still applies.

## Consequences

- NVIDIA users on Linux use less CPU for video (by how much isn't measured yet).
- Tested on one GPU and one driver. Freezes after a jump on other setups would be the sign to
  revisit this; the TODO keeps a real-desktop measurement.
- `preferences.json` files that already say `"gpuDecode": false` keep it off: only files without
  the setting take the new default. The setting is unreleased, so that only affects testers.
