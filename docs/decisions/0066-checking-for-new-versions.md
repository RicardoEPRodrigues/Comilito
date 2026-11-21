# 0066. Check GitHub for a new version at start, and point to the website

- Status: Accepted
- Date: 2026-11-21

## Context

Comilito is downloaded from the website (the AppImage and the MSI, decision 0064), and nothing
tells users when a new version is out. Real in-app updates (Tauri's updater plugin) need a
signing key and an update manifest published with each release; that's for later (TODO.md,
"In-app auto updates").

## Options considered

- **Tauri's updater plugin now**: downloads and installs updates, but needs a key to keep safe
  and a `latest.json` per release.
- **Ask GitHub's API for the newest release** and tell the user, leaving the download to them.
  No new dependency (`ureq` and `serde_json` are already used), and the releases are already
  there (decision 0063).

## Decision

Once per run, after the window is ready, `updates.rs` asks
`api.github.com/repos/RicardoEPRodrigues/Comilito/releases/latest` for the newest published
release (drafts and pre-releases never count), and compares its tag (`v1.1.0`) with this
version, numerically. If it's newer, the UI shows a notice that stays until closed:
"Comilito 1.1.0 is available", with a **Download** button that opens
`https://comilito.colorcrow.me/#download`. Offline, a GitHub error or no release: nothing is
shown, and the reason goes to the log.

It's **on by default**, with a switch in Preferences › Behaviour (`checkForUpdates`). The request
tells GitHub the computer's IP address and Comilito's version (in the User-Agent), nothing else.

## Consequences

- Users hear about new versions, and download them from the website as before.
- Unauthenticated, GitHub allows 60 requests an hour per IP address; one per run is far below.
- The repository's name is in the code: renaming or moving it needs a new version.
- Moving to real updates later replaces the notice's button, not the check.
