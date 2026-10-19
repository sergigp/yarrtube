# CLAUDE.md

## What this is

Yarrtube tracks YouTube playlists and channels and downloads their videos to
a local directory via `yt-dlp`. It's packaged as a Docker image meant to run
continuously (e.g. on a NAS): a `serve` daemon exposes an HTTP API plus an
embedded React web UI, syncs tracked sources on an interval, lays each
tracked source out as a TV show (channels: `Season <year>/S<year>E<MMDDii> -
<title>.*` by publish date; playlists: `Season 1/S01E<nnn> - <title>.*` in
playlist order; with NFO metadata) for Plex/Kodi/Jellyfin, and optionally asks Plex to scan a
show folder after each download. `yarrtube migrate-layout` moves a library
from the old per-video-folder layout.

Docs: `doc/INSTALLATION.md` (Docker, Compose, every environment variable),
`doc/DEVELOPMENT.md` (building from source), `doc/PLEX.md` (Plex TV Shows
library, scan trigger), `doc/UPGRADING-TO-0.9.0.md` (layout migration
from 0.8.x, temporary), `doc/ARCHITECTURE.md`. Every merge to `main` that
passes CI publishes the multi-arch image to `ghcr.io` as `:edge` +
`:sha-<short sha>` via `.github/workflows/release.yml`, with no git tag
and no `:latest`. Stable releases (`:latest`, `:X.Y.Z`, `:X.Y`) are cut
only by pushing a `vX.Y.Z` tag by hand; the tag is the version
(`Cargo.toml`'s isn't used). See "Deploy" in `doc/DEVELOPMENT.md`.

## Commands

```bash
cargo build --release                 # build
cargo test --locked                   # run all tests
cargo test <substring>                # run a single test / module by name filter
cargo fmt --all -- --check            # formatting check (CI enforces this)
cargo clippy --all-targets --all-features --locked -- -D warnings   # lint (CI enforces this, warnings fail)
```

Web UI (in `web/`, TypeScript + Vite + Vitest):

```bash
npm run check       # typecheck + lint + test, what CI runs
npm run test        # Vitest unit/component suite (fast, jsdom, mocked API)
npm run test:watch  # same suite, re-running on change
npm run typecheck   # tsc, strict mode
npm run build       # produces dist/, embedded into the binary by cargo build
```

## Frontend testing (web/)

Tests are colocated with what they test (`foo.ts` → `foo.test.ts`) and run in
jsdom with Vitest + Testing Library. The whole suite must stay fast (seconds)
and deterministic — anything needing the real backend or real downloads
belongs in `smoke-tests/`, not here.

The one mocking rule: **only the network boundary (`fetch`) is mocked** — via
`mockApi` from `src/test/helpers.tsx`, a route table keyed by
`"METHOD /api/path"` (query string included). Unrouted requests throw, so
declare every endpoint a test touches. A route value can be JSON, an
`{ status, error }` HTTP error, a function computing either per call, or
`pendingForever()` to hold a query in its loading state. Never mock
components, hooks, react-query, or the router.

Conventions:

- Render routed/query components with `renderWithProviders` (real
  `QueryClient` with retries/polling off + `MemoryRouter`); pure presentational
  components can use plain `render`.
- Build API fixtures with the helpers' builders (`aChannel`, `aPlaylist`,
  `aVideo`, `aHomeVideo`, `aTask`) and override only the fields the test is
  about.
- Drive the UI with `@testing-library/user-event` and query by role/label,
  the way a user would; assert exact user-visible text, not implementation
  details.
- Time-sensitive logic (debounce, relative timestamps, task categories) uses
  `vi.useFakeTimers()` + `vi.setSystemTime(...)`; restore real timers in
  `afterEach`.
- Pure logic lives in `src/lib/` precisely so it can be unit tested — when a
  component grows non-trivial data logic, extract it there and test it as
  plain functions.
- jsdom gaps (`matchMedia`, `ResizeObserver`, `localStorage`, pointer
  capture) are stubbed once in `src/test/setup.ts`; add new global stubs
  there, not in individual tests.
- API response types in `src/api/types.ts` mirror the Rust DTOs under
  `src/application/http/*/dto.rs` — when a backend DTO changes, update the
  type and the fixture builder together.

## Local testing

To test local changes, run `scripts/run-local.sh`. It rebuilds the web UI,
loads `.env` and starts `cargo run -- serve`. The SQLite database is created
in the repository root (`yarrtube.sqlite3`) and videos are downloaded into
`videos/`. The necessary env variables should already be loaded in the shell.

## Spec-driven development (OpenSpec)

This repo uses OpenSpec (`openspec/`) for spec-driven change management —
proposals and specs live under `openspec/changes/` and `openspec/specs/`
(one directory per capability). Use the
`openspec-*` / `opsx:*` skills (propose, apply, update, sync-specs, archive,
explore) when starting, continuing, or finalizing a spec'd change rather than
editing `openspec/` files by hand.
