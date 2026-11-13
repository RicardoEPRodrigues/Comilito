# 0017. Presets get their own tab; sounds become a collapsible Library drawer

- Status: Accepted (presets tab, "Library" name, presets keep disabled scenes);
  Proposed (drawer layout, the name "Tapestry", in-place editors)
- Date: 2026-11-13

## Context

- Presets sat above scenes in the left panel, but they're mostly for preparing a session, not
  for use during play.
- Sounds lived in a tab next to Scenes. Adding a sound mid-session meant switching tabs, and a
  big collection meant a very long list.
- "Live stack" doesn't read well as a name.

## Decision

Accepted (owner's request):

- The left panel has two tabs, **Scenes** and **Presets**. Presets can contain disabled scenes
  (the data already stores `enabled` per preset entry), shown faded with an unchecked box.
- "Sounds" is renamed **Library**.

Proposed (see screens D, D2 and F in `pencil/comilito.pen`):

- **Library drawer:** the Library becomes a drawer along the bottom of the window, above the mixer.
  - **Expanded:** three columns (Music, Ambience, Effects), each grouped by the folders inside the
    project's `audio/<category>/`, with folders that fold away.
  - **Collapsed:** a single bar with a search field (`/` focuses it). Results appear in a popover
    and can be dragged straight onto a scene.
  - Toggled with its chevron or a shortcut.
- **Drag targets:** a sound can be dropped on a scene in the Scenes tab or on an entry in the stack.
- **Large libraries:**
  - search by name or `#tag`
  - quick filters (All, Favourites, Recent, In tapestry, Unused)
  - tags
  - collapsible folders
  - three columns side by side instead of one long list
  - a virtualised list in the implementation, so thousands of rows stay smooth
- **Name for the stack:** **the Tapestry**: scenes are threads woven together, top over bottom.
  Alternatives: *Soundstage*, *Now Playing*.
- **In-place editors** (Unity-prefab style): scenes, presets and sounds open in the centre panel
  under a context bar (`Library › Music › Barovia › Road to Barovia`) instead of in pop-ups.
  The live mix keeps playing.

## Consequences

- Sounds gain optional `tags` and `favourite` fields, and "recent" needs a small usage record. All
  are additions to `comilito.json`, so `formatVersion` and docs/save-format.md must be updated when built.
- Library folders map to real subfolders of `audio/<category>/`, so moving a sound between folders
  moves its file.
- Modal editors (SceneEditor, SoundEditor) are replaced by in-place views once the layout is accepted.

## Follow-up (same day, owner feedback on screens D2/D3)

- **Scene groups and preset groups are collapsible.** The "Scenes" subtitle is dropped. Each
  group heading shows a chevron and a count.
- **Preset groups are editable.** Each group heading has a menu (rename, delete), and there's a
  "New group" action. Each preset has a menu: Load, Edit, Rename, Duplicate, **Move to group ▸**
  (existing groups, Ungrouped, New group…) and Delete. Presets can also be dragged onto a group heading.
- A preset card's meta text ("5 scenes · 2 disabled") sits under the name and wraps, so it
  never overflows the card.
- **Loading a preset over an unsaved tapestry asks first.** The tapestry header shows
  "Not saved in a preset" when the current tapestry doesn't match any preset. Loading a preset
  then opens a dialog with:
  - a preview of the current tapestry
  - name and group fields
  - buttons: *Cancel* / *Replace without saving* / *Save and load*
  - "Don't ask again", controlled by a Behaviour preference (ask / always replace)
