# 0042. Open projects from outside the projects folder, in place

- Status: Accepted (adds to [0023](0023-home-screen-and-projects-folder.md))
- Date: 2026-11-16

## Context

The home screen lists the folders inside the projects folder, so a project anywhere else (on
another drive, in a synced folder, shared with another DM) could only be brought in by
importing a copy. The owner asked to open such projects where they are, without importing.

## Options considered

- **Import (copy) only, as before:** two copies drift apart; large audio folders double.
- **Use the recent list to show them:** no new data, but it holds only the last 20 opened, so
  a project from elsewhere would vanish from the home screen once it fell off the list.
- **A separate list of projects from elsewhere:** one more preference, but they stay listed
  until the user removes them.

## Decision

- *Open a project folder…* in the home screen's More menu picks a folder with a
  `comilito.json` and opens it in place. A folder without one is refused with a plain message.
- Rust remembers every project opened from outside the projects folder in `otherProjects`
  (`preferences.json`), written only by Rust like `recentProjects`. "Outside" means not
  directly inside the projects folder (comparing resolved paths, so a symlink or another
  spelling counts as the same folder).
- The home screen lists them with the others, showing the folder they're in. Their menu adds
  *Remove from list*, which forgets them and leaves the files alone. Deleting a project also
  forgets it. A duplicate of one is made next to it, so it's from elsewhere too.
- A remembered folder that isn't there (an unplugged drive) is left out of the list but kept,
  so it reappears when the drive comes back.
- New projects are still created in the projects folder. Moving the projects folder (0023)
  doesn't touch projects from elsewhere.

## Consequences

- Backups are filed by project name (0026), so a project from elsewhere with the same name as
  one in the projects folder shares its backup folder, and each one's new backups count
  against the other's "keep" limit: one can push out the other's older backups. That was
  already true of two projects with the same name in the projects folder; this makes it more
  likely. Noted in TODO.md.
- If the user changes the projects folder with *Choose…*, projects in the old one stop being
  listed unless they were opened from elsewhere, as before.
