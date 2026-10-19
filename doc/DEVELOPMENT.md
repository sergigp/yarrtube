# Building from source

Docker is the supported way to run Yarrtube (see
[INSTALLATION.md](INSTALLATION.md)). This guide is for building and running
the binary directly, e.g. to hack on it.

## Requirements

- Rust (stable, 1.85+; the crate uses edition 2024)
- Node.js 22 and npm, to build the web UI
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your `PATH`
- [ffmpeg](https://ffmpeg.org/), which yt-dlp uses to merge audio and video
- [Deno](https://deno.com/), which yt-dlp uses to solve YouTube's player
  challenges. Without it some videos fail with "This video is not available"
- A YouTube Data API v3 key (see [INSTALLATION.md](INSTALLATION.md#requirements))

## Build

The web UI is embedded into the binary at compile time, so build it first.
`cargo build` fails if `web/dist/` is missing:

```bash
(cd web && npm ci && npm run build)   # produces web/dist/
cargo build --release                 # produces target/release/yarrtube
```

## Run

```bash
cp .env.example .env    # then fill in YOUTUBE_API_KEY
scripts/run-local.sh
```

`run-local.sh` rebuilds the web UI, loads `.env`, points `YTDLP_PATH` at the
`yt-dlp` on your `PATH`, and starts `yarrtube serve` on
<http://localhost:8080>. The SQLite database (`yarrtube.sqlite3`) and
downloaded videos (`videos/`) live in the repo root, and both are gitignored.
Pass `--skip-web-build` to reuse the existing `web/dist/`, or `--help` for the
other options.

To run the binary yourself, set at least `YOUTUBE_API_KEY`, `YTDLP_PATH`
and `YARRTUBE_VIDEOS_PATH` (their defaults, `/app/bin/yt-dlp` and `/videos`,
only make sense in the Docker image) and run `target/release/yarrtube serve`.
All variables are listed in [INSTALLATION.md](INSTALLATION.md#configuration);
that table shows the image's defaults, but outside Docker the database and
avatars default to `yarrtube.sqlite3` and `avatars/` in the working directory.

## Web UI

The UI in `web/` is a React + Vite single-page app written in TypeScript.
To iterate on it with hot reload, keep the backend running
(`scripts/run-local.sh --skip-web-build`) and start Vite's dev server, which
proxies `/api` and `/media` to `localhost:8080`:

```bash
cd web
npm run dev          # dev server with hot reload
npm run test:watch   # Vitest suite, re-running on change
npm run check        # typecheck + lint + test, what CI runs
```

Layout of `web/src/`:

- `api/`: the HTTP layer. Response types mirroring the Rust DTOs
  (`types.ts`), fetch wrappers (`client.ts`) and react-query hooks
  (`queries.ts`).
- `lib/`: pure, unit-tested helpers (formatting, slugs, sidebar ordering,
  task descriptions).
- `hooks/`: React hooks (debouncing, the add-dialog lookup flow, video
  selection, watch-progress reporting).
- `components/`: the app's components; `components/ui/` holds the shadcn/ui
  primitives.
- `test/`: test setup and helpers. `mockApi` stubs `fetch` with a route
  table, `renderWithProviders` wraps a component in react-query and a memory
  router, and `aChannel`/`aVideo`/… build API fixtures.

Tests sit next to what they test (`foo.ts` / `foo.test.ts`) and run in jsdom
against the mocked API, so the suite finishes in seconds.

## Checks

These are what CI runs:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
(cd web && npm run check)   # typecheck + lint + tests
```

End-to-end tests against a real container live in `smoke-tests/` (see its
README) and run with `scripts/run-smoke-tests.sh`.

## Deploy

Images are published to `ghcr.io/sergigp/yarrtube` by
`.github/workflows/release.yml`, on two channels.

### Edge: every merge to `main`

Every merge to `main` that passes CI publishes `:edge` and
`:sha-<short sha>`. No git tag is created and `:latest` doesn't move, so
users on `:latest` never get an untested merge. The maintainer's NAS runs
`:edge` and pulls it every 5 minutes, so a merge is live there a few
minutes after CI finishes. In the web UI, an edge build shows an `edge`
badge next to the logo. Click it to see the commit it was built from.

`:edge` only follows `main`'s head: a build that finishes after a newer
merge has landed still publishes its `:sha-<short sha>` but leaves `:edge`
to the newer build.

If an edge build breaks, roll back by pinning the previous commit's image
(`image: ghcr.io/sergigp/yarrtube:sha-<short sha>`) until a fix is merged,
then switch back to `:edge`.

Running the Release workflow by hand (Actions → Release → Run workflow)
builds any ref and publishes only `:sha-<short sha>`. Use it to try a
branch on the NAS without touching `:edge` or `:latest`.

### Stable: a pushed `vX.Y.Z` tag

A stable release happens only when you push a `vX.Y.Z` tag. It publishes
`:latest`, `:X.Y.Z` and `:X.Y`. Once `:edge` has been tested, tag the
commit it was built from and release it:

```bash
gh release create vX.Y.Z --target <sha> --generate-notes
```

(or `git tag vX.Y.Z <sha> && git push origin vX.Y.Z`; the GitHub release
then has to be written by hand). The tag is the version: `Cargo.toml`'s
version isn't read by anything and doesn't need bumping.
