# 0020. Crop sounds in the app, saving a copy or overwriting

- Status: Accepted
- Date: 2026-11-13

## Context

Downloaded and imported audio often has silence, intros or talking at the ends, and ambience is
often far longer than needed. The owner wants to crop sounds inside Comilito and choose between
overwriting the original and saving a copy.

## Options considered

- **Non-destructive trim points** (store start/end in `comilito.json`; kira plays only that region):
  no new files, but the file on disk stays long and other tools don't see the crop. Not what was asked.
- **Decode in Rust and write WAV:** exact cuts, but uncompressed files are about 10× larger.
- **ffmpeg** (already managed by Comilito for downloads, 0019): cuts the file in its own format.

## Decision

Cropping writes a new file with **ffmpeg**. The UI is screen F3 in `pencil/comilito.pen`, opened
with the **Crop** button on the editor's waveform.

- **Selection:** a large waveform with draggable start/end handles (zoom in/out/fit), plus exact
  time fields (Start, End, Length).
- **Preview:** plays the selection. **Loop seam** plays the last and first seconds back to back,
  to check that a loop is smooth.
- **Options:**
  - *Fade edges* (on, 10 ms): avoids clicks at the cut.
  - *Snap to quiet points* (on): moves a handle to the nearest low-volume moment, within a short distance.
  - *Keep exact cut* (off): by default the audio is copied without re-encoding, so there's no
    quality loss, and the cut lands on the nearest audio frame (a few milliseconds). Turning this
    on re-encodes in the same format at high quality for a sample-exact cut. Fading the edges also
    re-encodes the first and last moments.
- **Saving:**
  - **Save as a copy** (default): a new sound named "<name> (cropped)" in the same folder, with the
    same settings and tags. Scenes keep using the original.
  - **Overwrite the original:** the file is replaced. The original goes to the **system trash**
    (consistent with 0007), and every scene using the sound gets the cropped version. The sound
    keeps its id, so scenes and presets don't change.
- After saving, the hash and loudness are measured again for the new file (background workers).

## Consequences

- Cropping needs ffmpeg, so the first crop may trigger the tool download (0019).
- A downloaded sound's `source.range` describes the download, not later crops. A copy keeps the
  `source` of its original.
- The waveform needs peak data. It's computed once per file in Rust (decoding with symphonia),
  cached next to the loudness data, and sent to the UI as a small array.
