# Decision records

Short records of decisions made while planning and building Comilito, with the reasons behind them.
They're never rewritten: if a decision changes, a new record supersedes the old one.

## Index

| # | Decision | Status | Date |
|---|----------|--------|------|
| [0001](0001-tech-stack-tauri.md) | Use Tauri 2 with Rust and Svelte | Accepted | 2026-11-12 |
| [0002](0002-audio-in-rust.md) | Play all audio in Rust (kira), not the webview | Accepted | 2026-11-12 |
| [0003](0003-start-from-scratch.md) | Start from scratch instead of forking Ascapes Mixer | Accepted | 2026-11-12 |
| [0004](0004-save-format.md) | `.comilito` zip bundle with optional audio and relative paths | Accepted | 2026-11-12 |

## Template

```markdown
# NNNN. Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
What problem or question led to this decision.

## Options considered
- Option A: pros / cons
- Option B: pros / cons

## Decision
What we chose.

## Consequences
What this makes easier, harder, or commits us to.
```
