# yarrtube web UI

A small React + Vite single-page app, written in TypeScript, that browses
playlists, a playlist's videos, and pending/in-progress tasks. Built with
`npm run build` and embedded into the `yarrtube` binary at compile time —
see `DEVELOPMENT.md` at the repo root.

```bash
npm ci
npm run dev        # local dev server, proxies /api to localhost:8080
npm run build      # produces dist/, embedded by `cargo build`
npm run test       # Vitest unit/component suite (jsdom, no backend needed)
npm run test:watch # the same suite, re-running on change
npm run typecheck  # tsc, strict mode
npm run lint       # oxlint
npm run check      # typecheck + lint + test, what CI runs
```

## Layout

- `src/api/` — the HTTP layer: response types mirroring the Rust DTOs
  (`types.ts`), fetch wrappers (`client.ts`), and react-query hooks
  (`queries.ts`).
- `src/lib/` — pure helpers (formatting, slugs, sidebar ordering, task
  descriptions). Everything here is plain data-in/data-out and unit tested.
- `src/hooks/` — React hooks: debouncing, the add-dialog lookup flow, video
  selection in detail views, and watch-progress reporting.
- `src/components/` — the app's components; `components/ui/` holds the
  shadcn/ui primitives.
- `src/test/` — test setup and helpers: `mockApi` stubs `fetch` with a
  route table, `renderWithProviders` wraps a component in react-query and a
  memory router, and `aChannel`/`aVideo`/… build API fixtures.

Tests live next to what they test (`foo.ts` / `foo.test.ts`). They run in
jsdom against the mocked API, so the whole suite finishes in seconds; the
slow end-to-end checks against the real Docker image remain in
`smoke-tests/` at the repo root.
