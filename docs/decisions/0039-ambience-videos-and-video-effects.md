# 0039. Ambience videos and video effects are kept apart, with Library tabs

- Status: Accepted (amends [0035](0035-video.md): "one Video category in the Library")
- Date: 2026-11-15

## Context

Decision 0035 had one Video category: any video could go in a scene's playlist or on its video
effects pad. Sounds don't work that way: music, ambience and effects are separate categories,
each going in its own place. The owner wants videos to behave the same, consistently. With
two video categories the Library would have five columns (Music, Ambience, Effects, Ambience
videos, Video effects), too many for the drawer. The design is screens 9 to 11 of
`pencil/video-player.pen`.

## Options considered

- **Keep one Video category:** simple, but inconsistent with sounds.
- **Two video categories in a fifth and sixth column:** consistent, but cramped.
- **Two video categories behind a Sounds / Videos tab:** consistent, and each tab has at most
  three columns.

## Decision

- Each video has a **kind**, like a sound's category: `ambience` (a scene's playlist) or
  `effect` (the video effects pad). Files live in `video/ambience/` and `video/effects/`
  (with folders), like `audio/<category>/`. Changing a video's type moves its file and takes it
  out of scene slots of the other kind, with the same warning as changing a sound's category.
- **The Library gets Sounds / Videos tabs**, in the workspace's left column and in the drawer
  (each remembered on its own). Sounds is as before. Videos has its own smart lists (All,
  Favourites, Recent, Unused, May not play, Missing files) and the two kinds with their
  folders; the drawer shows them as two columns. The foldable Video column goes.
- Scenes take each kind in its own place: the Ambience videos section and the Video effects
  section each list only their kind; a video dragged onto a scene goes where its kind goes.
- **Existing projects:** every video becomes an ambience video, and its file moves into
  `video/ambience/` (keeping its folders) the first time the project opens. A video already
  used as a scene's video effect isn't taken out: it stays, marked "change its type" in the scene
  editor, and still plays, so nothing is lost while the owner sorts them out.

## Consequences

- Save format: an optional `kind` on each video (missing: ambience); no `formatVersion` change.
  Older versions of the app still find the videos (their paths are stored) but show both kinds
  together, and would put new videos straight in `video/`; this version moves those into
  `video/ambience/` the next time the project opens.
- Video files dropped straight into `video/` (not into a kind's folder) while a project is open
  are picked up the next time it opens.
