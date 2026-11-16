# 0040. Videos crossfade back to their start unless marked as seamless loops

- Status: Accepted
- Date: 2026-11-15

## Context

A scene with one video loops it. The video window jumped from the last frame straight back to
the first, which is a visible cut for most videos (ones found online are rarely made to loop).
Videos that *are* made to loop would look worse with a fade, since their end already runs into
their start.

## Options considered

- **Always jump back:** right for seamless videos, a cut for everything else.
- **Always crossfade:** no cut, but blurs videos that loop perfectly.
- **A flag per video, crossfade by default:** the default suits most videos; seamless ones are
  marked once in the Library.
- **Fade to black and back:** simpler, but a dark flash every loop draws more attention than a
  crossfade, and the window already has two layers for crossfading between playlist videos.

## Decision

Each video has an optional `seamlessLoop` flag (save format: omitted when false, so older files
load unchanged and mean "crossfade"). The *Loops seamlessly* switch on a video's page in the
Library sets it, for ambience videos (effects play once). A lone video that isn't seamless is
started again on the window's other layer `crossfade + 0.5 s` before its end and faded into,
using the crossfade time from Preferences › Video. Videos shorter than twice that lead jump back
instead, so the fade doesn't take up most of the video.

## Consequences

- The last half second or so of a non-seamless video is covered by the fade.
- A video that loads slowly (streamed, on Linux) may reach its end before the other layer is
  ready; its last frame then stays until the fade starts.
- Playlists of several videos are unchanged: they already crossfade to the next video.
