# Smoke tests

An end-to-end Playwright suite that drives the real Docker image through a
browser: creating a playlist/channel, downloading a real video via yt-dlp,
and playing it back. Run it locally, e.g. before merging or around a large
refactor. It doesn't run in CI: GitHub-hosted runners' shared IPs get
YouTube's "confirm you're not a bot" check, so the workflow is manual-only.

Run it with:

```bash
./scripts/run-smoke-tests.sh
```

from the repository root. It builds the release Docker image, runs it with
a fresh temp database/videos directory, waits for it to become ready, then
runs this Playwright suite against it. Before that, it runs the upgrade
check (see below) unless `SMOKE_UPGRADE_FROM_IMAGE=none`. The container and temp directories
are always removed afterward; on failure, the container's logs are saved to
`smoke-tests/container.log` and Playwright's traces/videos are kept under
`smoke-tests/test-results/`.

## Fixture prerequisites

These tests hit the real YouTube Data API and download real videos — no
test doubles. That requires:

- **`YOUTUBE_API_KEY`**: a YouTube Data API v3 key. Locally, put it in the
  repository root's `.env` (same as `scripts/run-local.sh` uses). In CI it
  comes from the `YOUTUBE_API_KEY` repository secret (Settings > Secrets
  and variables > Actions > New repository secret) — never printed or
  logged.
- **`SMOKE_PLAYLIST_ID`**: the id of a public, maintainer-owned YouTube
  playlist containing the Creative-Commons "Big Buck Bunny" video. If the
  playlist's contents ever change, the playlist test's download assertions
  may need to change with it.
- **`SMOKE_VIDEOS_DIR`**: the host directory mounted as the daemon's videos
  root, so specs can check the files on disk. `run-smoke-tests.sh` sets it
  to its temp directory.

- **`SMOKE_PLEX_STUB_URL`**: where specs reach the stand-in Plex server
  (`plex-stub.js`) the daemon is pointed at via `YARRTUBE_PLEX_*`.
  `run-smoke-tests.sh` starts it on `SMOKE_PLEX_STUB_PORT` (default
  `18324`) and configures the container for it.

The suite refuses to start without `SMOKE_PLAYLIST_ID`, `SMOKE_VIDEOS_DIR`
and `SMOKE_PLEX_STUB_URL`, rather than skipping the specs that need them.

Optional overrides (defaults shown):

| Env var                      | Default                 |
| ----------------------------- | ------------------------ |
| `SMOKE_PLAYLIST_ID`           | `PLXWRoRTUXjks` (https://www.youtube.com/playlist?list=PLXWRoRTUXjks) |
| `SMOKE_PLAYLIST_NAME`         | `test` (the playlist's YouTube title) |
| `SMOKE_CHANNEL_HANDLE`        | `@BlenderOfficial`      |
| `SMOKE_CHANNEL_VIDEO_LIMIT`   | `1`                      |
| `YARRTUBE_PORT`               | `8080`                   |
| `SMOKE_READY_TIMEOUT_SECONDS` | `120`                    |
| `SMOKE_RETRY_BASE_DELAY_SECONDS` | `10`                  |
| `SMOKE_PLEX_STUB_PORT`       | `18324`                  |
| `SMOKE_UPGRADE_FROM_IMAGE`   | `ghcr.io/sergigp/yarrtube:latest` (`none` skips the upgrade check) |

## What every test gets

Specs import `test`/`expect` from `helpers/fixtures.js`, not
`@playwright/test`. Each test starts from an empty library (leftovers from
an earlier failed test are deleted through the API), and fails on an
uncaught page error or a 5xx `/api` response.

## What's covered

- **Upgrade** (`upgrade/`): the released image (`SMOKE_UPGRADE_FROM_IMAGE`)
  adds the fixture playlist through the API, downloads it and records a
  watch position (`seed.spec.js`); it is stopped and this build starts on
  the same database and videos (`verify.spec.js`), which must list the
  playlist with its settings, keep the video downloaded (nothing queued to
  download again) with its position, play it from the playlist's URL and
  keep its next sync scheduled. This is also the suite's restart check.

- **Playlist lifecycle** (`tests/playlist.spec.js`): add the fixture
  playlist via the Add dialog, wait for its video to download, verify it
  shows up (with thumbnail + duration) in the playlist detail view and the
  home feed and on disk as a Plex episode (`Season 1/S01E…` media, `.nfo`,
  thumbnail, `tvshow.nfo`) and that Plex was asked to scan its folder in
  the section holding it, play it back, sync it (changing nothing), run
  its sync from the Tasks view, hit the duplicate-path error, then delete
  it and check its folder is removed.
- **Channel lifecycle** (`tests/channel.spec.js`): same flow for the
  fixture channel (episodes under `Season <year>`, plus a `poster.jpg`),
  including marking a single video watched, with loose (non-title)
  assertions since the channel's newest video can change, plus the
  invalid-handle error case.
- **Channel video limit** (`tests/channelVideoLimit.spec.js`): a channel
  with two downloaded videos, edited down to one: the next sync drops the
  older video from the list and its files from disk, keeping the newer.
- **Mobile sidebar** (`tests/mobile-sidebar.spec.js`): opening/closing the
  sidebar at a narrow viewport.
- **Tasks view** (`tests/tasks.spec.js`): the recurring yt-dlp self-update
  task the daemon seeds at startup is listed as pending.

Together these are also the only coverage of the HTTP API's route wiring
(Rust handler tests call handlers directly, bypassing the router), so every
`/api` route must be exercised by at least one spec. This is enforced: the
browser's `/api` requests are recorded, and `routeCoverageReporter.js` fails
a passing full run (`SMOKE_CHECK_ROUTE_COVERAGE=1`, set by
`run-smoke-tests.sh`) if any route parsed from
`src/application/http/mod.rs` was never hit.

After the suite, `run-smoke-tests.sh` also fails the run if the container
logged a panic.

## Local development

```bash
npm --prefix smoke-tests ci
npx --prefix smoke-tests playwright install --with-deps chromium
```

Then start the Plex stub (`node smoke-tests/plex-stub.js`) and, against an
already-running yarrtube instance started with
`YARRTUBE_PLEX_URL=http://localhost:18324`, `YARRTUBE_PLEX_TOKEN=smoke-token`
and `YARRTUBE_PLEX_VIDEOS_PATH=/plex/yarrtube`:

```bash
BASE_URL=http://localhost:8080 \
  YOUTUBE_API_KEY=... \
  SMOKE_PLAYLIST_ID=... \
  SMOKE_VIDEOS_DIR=$PWD/videos \
  SMOKE_PLEX_STUB_URL=http://localhost:18324 \
  npm --prefix smoke-tests test
```
