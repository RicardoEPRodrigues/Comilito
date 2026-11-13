# 0021. Remove the Play / Edit mode; everything is always editable

- Status: Accepted
- Date: 2026-11-13
- Supersedes: the Play / Edit mode parts of [0008](0008-scenes.md) and [0018](0018-library-workspace.md)

## Context

Play and Edit modes were meant to stop accidental changes during a session: in Play mode, scenes
and sounds couldn't be edited. The design has since moved to in-place editing (0017, 0018) with
explicit Save / Discard, confirmations for destructive actions, trash instead of deletion,
backups (0007) and a save prompt before replacing an unsaved tapestry. With those in place, the
mode was mostly an extra switch to remember. Edits are also often wanted mid-session (add a sound
to a scene, fix a volume).

## Decision

Remove the Play / Edit toggle. Everything is editable at all times.

## Consequences

- The header loses the toggle. Editing actions (✎, 🗑, New scene, Add files, remove-from-scene ✕,
  rename project) are always shown.
- The per-project "last mode" stored in the webview's localStorage is no longer read or written.
  Old values are harmless and ignored.
- Accident protection comes from the other mechanisms: Save/Discard in editors, confirmations,
  trash, backups and the unsaved-tapestry prompt.
