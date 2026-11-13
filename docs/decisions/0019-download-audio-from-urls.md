# 0019. Download audio from video sites and direct URLs (no streaming)

- Status: Accepted
- Date: 2026-11-13

## Context

The owner wants to add audio from YouTube and other platforms, or from a plain URL to an audio
file. Audio must be **downloaded** into the project, never streamed. Sessions shouldn't depend on
a network connection, and project folders must stay self-contained (0006).

## Options considered

- **Write our own site extractors:** they break whenever a site changes. Not maintainable.
- **yt-dlp** (Unlicense, supports YouTube and 1,000+ sites, updated very often): the de-facto tool.
  It's an external program, so Comilito runs it as a separate process.
- **Direct links only:** simple, but doesn't cover video sites.

## Decision

- **Two sources, one dialog** ("Add audio from a URL", screen F2):
  - A **direct audio link** (`.mp3`, `.ogg`, `.flac`, `.wav`, `.m4a`) is downloaded by Rust over
    HTTP, then checked by opening it with the audio decoder before it's accepted.
  - **Any other page** is handed to **yt-dlp**, asking for audio only.
- **Format:** prefer formats our decoder (symphonia) plays without conversion: M4A/AAC first, then
  MP3/Vorbis. symphonia does **not** decode Opus, which is YouTube's other common audio format, so
  Opus-only sources are converted.
- **Part of a long video:** the dialog can download only a time range ("from 0:05:00 to 0:15:00").
  Long ambience videos run for hours, but a 10-minute loop is usually enough.
- **ffmpeg** is required for time ranges and conversions (yt-dlp uses it for both).
- **Comilito downloads and manages the tools itself** (owner's choice: more disk space, less
  work for the user):
  - The first time "From URL" (or cropping, 0020) needs them, Comilito downloads the tools for
    the platform into the app data folder, showing progress:
    - yt-dlp: its GitHub releases (standalone binaries for Linux, Windows and macOS).
    - ffmpeg: ffmpeg.org publishes no binaries, so Comilito uses the builds yt-dlp recommends
      (`yt-dlp/FFmpeg-Builds` on GitHub, Linux and Windows). macOS needs another source, decided
      when macOS support is added.
  - The SHA-256 of each download is checked against the checksums published with the release.
  - It never relies on a system install, so the version is known and the behaviour is the same everywhere.
  - Comilito checks for yt-dlp updates (at most once a day, when the dialog opens), since sites
    change often and an old yt-dlp stops working. The version is shown with a "check for updates" link.
  - Preferences › Projects & storage shows the tools' disk use, with a button to remove them.
- **Downloads queue:** downloads run in the background, with progress and cancel, shown in the
  Library workspace. The finished file is saved into `audio/<category>/<folder>/` and added like
  any other sound (hashing and loudness analysis follow).
- **Provenance:** each downloaded sound stores
  `source: { url, title, site, downloadedAt, range? }`. The editor shows it with *Open page* and
  *Download again*, and relinking a missing file can offer to re-download it.
- **Rights:** the dialog says plainly that the user must only download audio they have the right
  to use and must respect the site's terms. Comilito doesn't bypass DRM or paywalls.
- **No streaming:** nothing plays from the network. Audio is only ever played from the project folder.

## Consequences

- **Part of the first version.**
- External programs (yt-dlp, ffmpeg) become optional runtime dependencies, fetched on first use.
  The app works fully offline without them; only "From URL" for non-direct links, time ranges,
  conversions and cropping need them.
- About 400 MB of disk space once both are downloaded (measured 2026-11-13: the static ffmpeg and
  ffprobe builds are ~190 MB each; yt-dlp ~40 MB).
- A small HTTP client crate is needed for direct links and tool downloads (`ureq` or `reqwest`),
  to be justified when added.
- `comilito.json` gains the optional `source` field on sounds, which needs a `formatVersion` bump and
  an update to docs/save-format.md.
- Licences: yt-dlp is Unlicense. ffmpeg builds are LGPL/GPL depending on the build, and running them
  as separate programs (not linking) keeps Comilito's licence unaffected. Downloaded tools get
  credited in CREDITS.md / About.
