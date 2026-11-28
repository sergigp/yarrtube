# Development

Docker is the supported way to run Yarrtube (see
[INSTALLATION.md](INSTALLATION.md)). This guide is for working on it:
running it from source, iterating on the web UI and running the tests.

## Requirements

- Rust (stable, 1.85+; the crate uses edition 2024)
- Node.js 22 and npm, to build the web UI
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) on your `PATH`
- [ffmpeg](https://ffmpeg.org/), which yt-dlp uses to merge audio and video
- [Deno](https://deno.com/), which yt-dlp uses to solve YouTube's player
  challenges. Without it some videos fail with "This video is not available"
- A YouTube Data API v3 key (see [INSTALLATION.md](INSTALLATION.md#requirements))
- Docker, only for the end-to-end tests

## Run

```bash
cp .env.example .env    # then fill in YOUTUBE_API_KEY
scripts/run-local.sh
```

`run-local.sh` rebuilds the web UI, loads `.env` and starts a debug build of
`yarrtube serve` on <http://localhost:8080>. The SQLite database
(`yarrtube.sqlite3`) and downloaded videos (`videos/`) live in the repo
root, and both are gitignored. Pass `--skip-web-build` to reuse the existing
`web/dist/`, or `--help` for the other options.

To build the release binary by hand, build the web UI first: it's embedded
at compile time, and `cargo build` fails if `web/dist/` is missing.

```bash
(cd web && npm ci && npm run build)   # produces web/dist/
cargo build --release                 # produces target/release/yarrtube
```

To run it, set `YTDLP_PATH` and `YARRTUBE_VIDEOS_PATH`: their defaults
(`/app/bin/yt-dlp` and `/videos`) only exist in the Docker image. The
database, avatars and logs default to the working directory. All variables
are listed in [INSTALLATION.md](INSTALLATION.md#configuration).

## Web UI

The UI in `web/` is a React + Vite single-page app written in TypeScript.
To iterate on it with hot reload, keep the backend running
(`scripts/run-local.sh --skip-web-build`) and start Vite's dev server, which
proxies `/api`, `/media` and `/avatars` to `localhost:8080`:

```bash
cd web
npm run dev          # dev server with hot reload
npm run test:watch   # Vitest suite, re-running on change
npm run check        # typecheck + lint + test
```

Tests sit next to what they test (`foo.ts` / `foo.test.ts`) and run in jsdom
with only `fetch` mocked, so the suite finishes in seconds.

To try UI changes against a real library instead of an empty local one, point
the dev server at a running daemon, e.g. on a NAS:

```bash
YARRTUBE_REMOTE_URL=http://192.168.1.142:8080 scripts/run-spa-against-remote.sh
```

Only frontend changes show up. It's read-only by default, so actions like
mark watched or delete fail instead of changing the remote library; pass
`--allow-writes` to forward them.

## Checks

One script runs every test suite, locally and in CI. It takes one flag per
suite, and needs at least one:

| Flag         | Runs                                                                                       |
|--------------|--------------------------------------------------------------------------------------------|
| `--lint`     | The web UI's typecheck and oxlint, `cargo fmt --check`, clippy                             |
| `--arch`     | The architecture tests: the written conventions checked over the source                   |
| `--web`      | The web UI's unit tests (Vitest)                                                           |
| `--rust`     | `cargo test`, the architecture tests left out                                              |
| `--contract` | The [contract tests](#contract-tests): adapters against the real YouTube                   |
| `--e2e`      | The end-to-end suite: the Docker image driven through real browsers against a fake YouTube |
| `--all`      | Every suite                                                                                |
| `--ci`       | Every suite but `--contract`, with failing end-to-end tests retried                        |

```bash
scripts/run-tests.sh --lint --rust         # flags combine
scripts/run-tests.sh --e2e --workers 2     # run 2 end-to-end tests at once (default 3)
```

The script lists missing tools up front, and installs or builds what the
repo produces itself (`node_modules`, the web UI build, the Playwright
browsers), so it runs as is in a fresh clone or worktree.

CI runs `--ci` on every pull request and push to `main` or a `release/**`
branch. It leaves the contract tests out because YouTube bot-checks CI
runners. A second CI job fails while `openspec/changes/` holds a change that
isn't archived.

### End-to-end tests

The suite lives in `e2e-tests/`; its [README](../e2e-tests/README.md)
covers the fake YouTube, what's tested, the variables that tune a run and
how to run Playwright by hand.

Before the suite comes the upgrade check: the latest released image tracks
a real playlist, then the image under test starts on the same data. It
needs a real `YOUTUBE_API_KEY`, so it's skipped without one, as in CI.

### Contract tests

The adapters that talk to YouTube (`yt-dlp` downloads, thumbnails,
captions, channel listings, and the Data API's playlists, videos, channels
and avatars) have tests against the real thing, in modules named
`youtube_contract_tests` next to each adapter. They're `#[ignore]`d, so
`cargo test` and CI skip them. Run them with
`scripts/run-tests.sh --contract`, which needs `YOUTUBE_API_KEY`, after
upgrading `yt-dlp` or when downloads start failing, to tell a change on
YouTube's side from a bug of ours.

Enforced by: cargo test · `architecture_tests::contract_tests_are_ignored`

The same flag runs the Anthropic adapter's contract test when
`YARRTUBE_ANTHROPIC_API_KEY` is set: one tiny call, billed to that key.
