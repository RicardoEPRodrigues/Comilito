# 0033. Resuming downloads after a restart, and warning when quitting while busy

- Status: Accepted
- Date: 2026-11-14

## Context

Downloads ([0019](0019-download-audio-from-urls.md)) lived only in memory: quitting lost the
queue, and a running yt-dlp kept going as an orphan process, writing into a cache folder nobody
read again. Quitting could also cut off a backup halfway (leaving a broken zip) or the
fingerprinting of audio files ([0005](0005-storage-location-and-hashing.md)). Preferences listed
a "Warn when quitting during hashing or backup" setting ([0012](0012-preferences.md)) that was
never built.

## Options considered

- **Save the queue inside the project folder:** travels with the project, but the project
  folder may be synced to other computers, and a download is about this computer.
- **Save the queue in the app data folder, resume at start:** simple, but a download may belong
  to a project that isn't open, and finishing it then loses its source information (the folder
  watcher adds it as a plain file) and "Download again" can't refill the sound.
- **Save the queue in the app data folder, resume when its project opens:** the download is
  added to the project as usual, with its source.
- **Start every resumed download over**, or **continue from the partial data**.

## Decision

- Unfinished downloads (queued or running) are written to `downloads.json` in the app data
  folder whenever one is added, finishes or is cancelled. At start it's read back; entries whose
  project no longer exists are dropped. Each waits until its project is opened, then rejoins the
  queue with the same id, so its partial data in the cache folder is found again.
- Partial data is continued, not thrown away: yt-dlp continues its own `.part` files, and direct
  links are written to `<name>.part` and continued with an HTTP `Range` request (if the server
  doesn't support it, the download starts over). Anything else left in the cache folder may be a
  cut-off conversion and is deleted. A part of a video is cut by ffmpeg, which can't continue,
  so it starts over.
- On exit (and on cancel), a running yt-dlp is stopped together with any ffmpeg it started:
  on Unix yt-dlp runs in a process group of its own and the whole group is killed; on Windows
  `taskkill /T` stops the process tree. The worker sees `AppState::quitting` and leaves the job
  alone instead of recording a failure.
- Closing the main window while downloads, fingerprinting, a backup or a projects-folder move
  are running asks first ("Keep running" / "Quit anyway"), with a list of what would be
  interrupted. The preference (`behaviour.warnOnQuit`, default on) turns the question off.
  Loudness measuring isn't listed: it's quick per file and simply starts again.

## Consequences

- A download for a project that is never opened again waits forever in `downloads.json`, until
  the project folder is deleted.
- "Remove all data and quit" exits straight away, without the warning.
