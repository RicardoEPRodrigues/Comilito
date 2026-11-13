# 0015. Loudness normalization with EBU R128

- Status: Accepted
- Date: 2026-11-13

## Context

Audio from different sources is mastered at very different levels: a commercial music track
might sit around −10 LUFS, a field recording of rain around −30. Moving from one track or
ambience to another then jumps in volume, and the DM ends up adjusting every sound by hand.

## Options considered

- **Peak normalization** (scale so the loudest sample hits full scale): simple, but peak says
  little about how loud something sounds. One click in a quiet recording defeats it.
- **RMS normalization**: closer, but treats all frequencies alike and counts silence.
- **EBU R128 integrated loudness (LUFS)**: weights frequencies like the ear, ignores silence
  (gating), and is what broadcasters and streaming services use. The `ebur128` crate implements
  it (pure Rust, MIT, maintained).
- **Rewriting the audio files** at the normalized level vs. **applying a gain at playback**:
  rewriting loses quality and changes the user's files; a playback gain is free and reversible.

## Decision

Measure each file's integrated loudness and sample peak once, in the background, with EBU R128,
and store them in its `file.loudness`. At playback, add a gain of `target − measured` dB on top
of the sound's volume.

- On by default for **music and ambience**, off for effects (short one-shots measure unreliably
  and are usually meant to be punchy). Toggles per category in the project defaults.
- Target **−18 LUFS** by default, editable per project.
- Turning down has no limit. Turning up is capped at **+12 dB** and at the file's **headroom**
  (so its loudest sample never passes full scale), so quiet files don't distort.
- Files are decoded with kira's own decoder (`StaticSoundData::from_file`), so no direct
  dependency on Symphonia's lower-level API.

## Consequences

- Adding sounds starts a short background measurement ("Measuring loudness…"); a sound plays at
  its raw level until measured, then glides to its normalized level over a second.
- A decoded file is held in memory while it's measured (about 100 MB for a 4-minute track).
- Volumes and duck levels keep their meaning, relative to an evened-out starting point.
- Debug builds optimize dependencies (`[profile.dev.package."*"]`), since decoding unoptimized
  is many times slower.
