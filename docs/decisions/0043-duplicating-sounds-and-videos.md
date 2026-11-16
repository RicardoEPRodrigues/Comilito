# 0043. Duplicating a sound or video copies its file

- Status: Accepted
- Date: 2026-11-16

## Context

The owner asked for a way to duplicate a sound or video in the Library, for example to keep
the same file with other settings (a quieter, non-looping version of an ambience) or as a
starting point for cropping or trimming.

## Options considered

- **The copy shares the original's file:** no extra disk space, but every library entry so far
  owns its file. Removing one would trash a file the other still uses, moving or relinking one
  would have to move the other, cropping or trimming "in place" would change both, and the
  missing-file and fingerprint logic all assume one entry per file. Each of those would need
  reference counting.
- **The copy gets its own copy of the file:** costs disk space (a lot for long videos), but the
  copy is an ordinary library entry and nothing else has to change.

## Decision

Duplicating copies the file beside the original as "<name> (copy).<ext>" (numbered if taken)
and adds a library entry right after the original with the same settings, named
"<name> (copy)". It's in no scene and has no "last used" date. The content's hash and loudness
are carried over, since the content is the same. The file is written under a hidden name and
renamed when complete, so the folder scan never picks up a half-copied file, and the copy is
made without holding the app's state lock, so a large video doesn't freeze the app.

*Duplicate* is in a sound's right-click menu (Library drawer), on the sound's page and on a
video's page. A sound or video whose file is missing can't be duplicated.

## Consequences

- Duplicating a long video doubles its disk use; the copy can be trimmed afterwards.
- If disk space becomes a concern, sharing files can be reconsidered in a later decision.
