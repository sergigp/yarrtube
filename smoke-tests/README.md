# Smoke tests

An end-to-end Playwright suite that drives the real Docker image through a
browser: creating a playlist/channel, downloading a real video via yt-dlp,
and playing it back. Runs in CI on every PR/push to `main`, and can be run
locally on demand (e.g. before/after a large refactor).

Run it with:

```bash
./scripts/run-smoke-tests.sh
```

from the repository root. It builds the release Docker image, runs it with
a fresh temp database/videos directory, waits for it to become ready, then
runs this Playwright suite against it. The container and temp directories
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

Optional overrides (defaults shown):

| Env var                      | Default                 |
| ----------------------------- | ------------------------ |
| `SMOKE_PLAYLIST_ID`           | `PLXWRoRTUXjks` (https://www.youtube.com/playlist?list=PLXWRoRTUXjks) |
| `SMOKE_PLAYLIST_NAME`         | `yarrtube smoke tests`  |
| `SMOKE_CHANNEL_HANDLE`        | `@BlenderOfficial`      |
| `SMOKE_CHANNEL_VIDEO_LIMIT`   | `1`                      |
| `YARRTUBE_PORT`               | `8080`                   |
| `SMOKE_READY_TIMEOUT_SECONDS` | `120`                    |
| `SMOKE_RETRY_BASE_DELAY_SECONDS` | `10`                  |

## What's covered

- **Playlist lifecycle** (`tests/playlist.spec.js`): add the fixture
  playlist via the Add dialog, wait for its video to download, verify it
  shows up (with thumbnail + duration) in the playlist detail view and the
  home feed, play it back, sync it, hit the duplicate-path error, then
  delete it.
- **Channel lifecycle** (`tests/channel.spec.js`): same flow for the
  fixture channel, with loose (non-title) assertions since the channel's
  newest video can change, plus the invalid-handle error case.
- **Mobile sidebar** (`tests/mobile-sidebar.spec.js`): opening/closing the
  sidebar at a narrow viewport.

Custom playlists are out of scope — there's no UI entry point for them yet.

## Local development

```bash
npm --prefix smoke-tests ci
npx --prefix smoke-tests playwright install --with-deps chromium
```

Then, against an already-running yarrtube instance:

```bash
BASE_URL=http://localhost:8080 \
  YOUTUBE_API_KEY=... \
  npm --prefix smoke-tests test
```
