# 0038. The video keeps playing while the sound is paused

- Status: Accepted (amends [0035](0035-video.md): "Space pauses it with the sound")
- Date: 2026-11-15

## Context

Decision 0035 paused the video window together with the sound when Space is pressed. In use, a
frozen picture on the table's screen looks broken, and the DM pauses the sound to talk, not to
stop the picture.

## Options considered

- **Pause both (as before):** consistent with the sound, but a still frame on the TV.
- **Keep the video playing on Space, blank it on Esc:** the picture stays alive while talking;
  Esc ("stop all") still means everything stops.
- **Ignore both:** the video would never stop with the transport, which Esc users wouldn't expect.

## Decision

Space pauses the sound only; the video keeps playing. Esc still blanks the video window. A
preference (Preferences › Video, `keepPlayingWhenPaused`, on by default) brings back the old
behaviour.

## Consequences

- While paused, "switch every N minutes" keeps counting (the video is playing).
- Video effects keep playing too.
