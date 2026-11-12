# 0002. Play all audio in Rust (kira), not the webview

- Status: Accepted
- Date: 2026-11-12

## Context

A Tauri app could play sound with HTML `<audio>` or the Web Audio API in the webview. On Linux
that webview is WebKitGTK, which decodes media through GStreamer. Which codecs work then depends
on the GStreamer plugins each distro installs, a common cause of "no sound" bugs. Behaviour also
differs between WebKitGTK (Linux), WebView2 (Windows) and WKWebView (macOS).

## Options considered

- **Webview audio** (what Ascapes Mixer does): no Rust code needed, but behaviour depends on the
  platform and fade timing relies on JavaScript timers.
- **Rust with rodio**: simple and popular, but basic. Fades and mixer buses would be built by hand.
- **Rust with kira**: made for game audio. It has mixer sub-tracks, tweened volume and fades,
  seamless looping and streaming, and decodes ogg/mp3/flac/wav itself (through symphonia).

## Decision

All playback goes through a Rust audio engine built on kira. The UI only sends commands.

## Consequences

- Playback is the same on every OS and doesn't depend on system codecs.
- Categories map directly onto kira mixer sub-tracks.
- More Rust code: the audio engine is the main Rust learning area.
- Android needs checking later (kira uses cpal, which supports Android through AAudio/Oboe).
