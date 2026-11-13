# 0013. Plain Svelte with Vite instead of SvelteKit

- Status: Accepted
- Date: 2026-11-12

## Context

`create-tauri-app`'s Svelte template now generates a SvelteKit project. SvelteKit is a framework
for websites: file-based routing (`src/routes/+page.svelte`), server-side rendering, and adapters
to choose where the site runs. In Tauri, SSR must be turned off and a static adapter used, so
most of what SvelteKit adds isn't used.

## Options considered

- **SvelteKit (template default):** follows the official template. But it adds concepts the app
  doesn't need (routes, layouts, adapters, `.svelte-kit/` generated files) for someone who is
  learning Svelte at the same time.
- **Plain Svelte + Vite:** one `index.html`, `src/main.ts` and `src/App.svelte`. Fewer files and
  concepts. Tauri documents this setup too. If several windows are needed later (for example
  Preferences), each can get its own HTML entry point.

## Decision

Plain Svelte 5 with Vite, no SvelteKit.

## Consequences

- No router. Views are switched by ordinary component state. A small router could be added
  later if it's ever needed.
- The frontend builds to `dist/`, which `tauri.conf.json` points to.
