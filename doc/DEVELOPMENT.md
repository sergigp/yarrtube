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
proxies `/api`, `/media` and `/avatars` to `localhost:8080`:

```bash
cd web
npm run dev          # dev server with hot reload
npm run test:watch   # Vitest suite, re-running on change
npm run check        # typecheck + lint + test, what CI runs
```

To try UI changes against a real library instead of an empty local one, point
the dev server at a running daemon, e.g. on a NAS. Only frontend changes show
up; backend changes need a daemon running an image that has them.

```bash
YARRTUBE_REMOTE_URL=http://192.168.1.142:8080 scripts/run-spa-against-remote.sh
```

It's read-only by default: the dev server answers every request but GET and
HEAD with 405, so actions like mark watched or delete fail instead of changing
the remote library. Pass `--allow-writes` to forward them. When node can't
reach the remote but curl can (macOS Local Network privacy blocks node from LAN
addresses until the terminal app is allowed under System Settings → Privacy &
Security → Local Network), the script bridges the proxy through curl by itself.

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

One script runs every test suite, locally and in CI:

```bash
scripts/run-tests.sh                 # everything
scripts/run-tests.sh --only e2e      # one suite: web, rust, contract or e2e
scripts/run-tests.sh --ci            # what CI runs (also when CI=true)
scripts/run-tests.sh --workers 2     # end-to-end tests at once (default 3)
```

The suites are the web UI's checks, Rust formatting, clippy and tests, the
YouTube contract tests and the end-to-end suite (`e2e-tests/`, see its
README: the Docker image driven through real browsers against a fake
YouTube). CI runs all of them except the contract tests, which need the real
YouTube, on every pull request and push to `main`. The script only checks:
it installs and builds nothing, so install the web UI's and `e2e-tests/`'s
dependencies (and the Playwright browsers) and build the web UI first. Each
end-to-end worker runs its own container on its own ports, so the suite runs
in parallel and worktrees can run it at the same time. Before running
anything it checks every selected suite's dependencies, listing all that are
missing, and prints what will run: the suites, CI mode (`--ci` or
`CI=true`: no contract tests, failing end-to-end tests retried) and the
end-to-end workers and upgrade check. Each phase's output
goes to a log rather than the terminal: a progress line while it runs, its
time and test counts once done, and on failure only the failing tests with
their errors, plus the path of the kept logs.

### YouTube contract tests

The adapters that talk to YouTube (`yt-dlp` downloads, thumbnails,
captions, channel listings, and the Data API's playlists, videos, channels
and avatars) have tests against the real thing, in modules named
`youtube_contract_tests` next to each adapter. They're `#[ignore]`d, so
`cargo test` and CI skip them; run them with
`scripts/run-tests.sh --only contract`. It needs `yt-dlp` and `ffprobe` on `PATH` (or `YTDLP_PATH`), the network,
and `YOUTUBE_API_KEY` (from the environment or `.env`), and takes about ten
seconds. Run it after upgrading `yt-dlp` or when downloads start failing,
to tell a change on YouTube's side from a bug of ours. The fixtures (a short
Creative-Commons clip, Big Buck Bunny, the project's "test" playlist and an
inactive Creative-Commons channel) are listed with their credits in
`src/infrastructure/shared/youtube_contract.rs`.

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
