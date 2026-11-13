# 0019. Download audio from video sites and direct URLs (no streaming)

- Status: Accepted (feature, no streaming); Proposed (tooling details)
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

## Decision (proposed details)

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
- **Tools are managed by Comilito:**
  - Use `yt-dlp` / `ffmpeg` from the system PATH if present.
  - Otherwise offer to download the official builds into the app data folder.
  - Show the yt-dlp version with a "check for updates" link, since sites change often and an old
    yt-dlp stops working.
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

- External programs (yt-dlp, ffmpeg) become optional runtime dependencies. The app works fully
  without them; only "From URL" for non-direct links needs them.
- A small HTTP client crate is needed for direct links and tool downloads (`ureq` or `reqwest`),
  to be justified when added.
- `comilito.json` gains the optional `source` field on sounds, which needs a `formatVersion` bump and
  an update to docs/save-format.md.
- Licences: yt-dlp is Unlicense. ffmpeg builds are LGPL/GPL depending on the build, and running them
  as separate programs (not linking) keeps Comilito's licence unaffected. Downloaded tools get
  credited in CREDITS.md / About.
