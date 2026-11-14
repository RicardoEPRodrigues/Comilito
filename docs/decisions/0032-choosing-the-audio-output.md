# 0032. Choosing the audio output

- Status: Accepted
- Date: 2026-11-14

## Context

A DM may want Comilito on other speakers than the rest of the computer: the table's speakers
while their own headset carries a voice call, or an HDMI output to a TV. Preferences listed
"Audio output device" since [0012](0012-preferences.md), but the engine always opened the
system default.

kira plays through `cpal` and accepts a `cpal::Device`. On Linux, cpal's default host is ALSA,
whose device list is long and technical (40 entries for three outputs here: `hw:`, `plughw:`,
`surround51:`, `dmix:`... per card), and opening a card directly goes around PipeWire or
PulseAudio, which usually hold it.

## Options considered

- **List ALSA devices, filtered:** no new code, but the names don't match the desktop's sound
  settings, and opening a card directly can fail or take it away from other apps.
- **cpal's `pipewire` host:** lists PipeWire's outputs, but needs libpipewire's development
  files to build and the library at run time.
- **cpal's `pulseaudio` host:** pure Rust (`pulseaudio` crate, MIT), speaks the PulseAudio
  protocol that PipeWire also serves. It lists exactly the outputs the desktop shows, with the
  same names. New crates: `pulseaudio`, `futures`, `enum-primitive-derive`; no system library.

## Decision

- Turn on cpal's `pulseaudio` feature (Linux only). Preferences › Playback lists the outputs
  from cpal's default host (PulseAudio when it's running, otherwise ALSA; on Windows and macOS
  their own) plus **System default**, which names the current default.
- `playback.outputDevice` in `preferences.json` holds the chosen output's id (for example
  `pulseaudio:alsa_output.usb-…-stereo`), or `null` for the system default.
- **System default** still plays through ALSA's `default` on Linux, as before. PipeWire or
  PulseAudio route it to the desktop's default output and move it when that changes, and the
  stream keeps a stable name ("ALSA plug-in [comilito]"), so the desktop remembers its volume.
  (cpal's PulseAudio host names every stream `cpal-pulseaudio-<process id>`.)
- A **chosen output** is opened through the PulseAudio host, with a fixed buffer of 512 frames.
  Left to its default, that host buffers about **2 seconds**, so every sound started late
  (measured with cpal's stream timestamps: 2.0 s by default, 43 ms with 512 frames; the ALSA
  path measured 48 ms). Found and fixed the same day.
- Changing the output rebuilds the audio engine: kira can't move a running mix. What was
  playing starts again from the beginning on the new output; a paused mix stays stopped.
- If the chosen output isn't connected at start, the system default is used (and logged); the
  choice is kept, and Preferences shows it as "(not connected)". If it's unplugged while
  playing, kira moves to the system default by itself, keeping the buffer size and what's
  playing.

## Consequences

- Outputs have the same names as in the desktop's sound settings.
- While a chosen output is in use, the desktop's volume mixer shows Comilito as
  `cpal-pulseaudio-<number>`, and doesn't remember its volume between runs.
- After unplugging, Comilito doesn't go back to the chosen output when it's plugged in again;
  choosing it again in Preferences (or restarting) does.
- Windows and macOS list their own devices through cpal; untested there.
