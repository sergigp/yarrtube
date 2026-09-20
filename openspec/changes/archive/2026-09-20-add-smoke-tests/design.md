## Context

See proposal.md - Why/What Changes. Relevant existing seams: `GET /status` (readiness probe), `YTDLP_PATH`/`YOUTUBE_API_KEY`/`YARRTUBE_DB_PATH`/`YARRTUBE_VIDEOS_PATH`/`YARRTUBE_PORT` env vars already read by `serve.rs`, and the background task executor polls every 5s so downloads start promptly after creation (no artificial wait needed beyond the download itself). The web app has no build tooling beyond Vite/npm; there is no TypeScript anywhere in the repo, so the new project uses plain JS to match.

## Goals / Non-Goals

**Goals:**
- One script, run identically by a developer locally and by CI, that builds the real Docker image and proves the golden paths work through a browser.
- Keep the real-network fixtures (playlist ID, channel handle) out of test code so the maintainer can change them without touching JS.
- Minimize real downloads: exactly two (one playlist video, one channel video) cover every Tier 1 + Tier 3 scenario.

**Non-Goals:**
- Cross-browser coverage (Chromium only).
- Docker layer caching / build-speed optimization in CI (plain `docker build` each run; revisit only if CI time becomes a problem).
- Testing custom playlists (no UI entry point; out of scope per proposal).

## Decisions

- **Single Playwright worker, no parallelism** (`fullyParallel: false`, `workers: 1`): all tests share one container/DB; parallel runs would race on shared sidebar/home state.
- **Sync and delete live inside the same lifecycle test as the create+download+play flow**, not separate tests, so each fixture (playlist, channel) is only downloaded once.
- **Duplicate-path error test reuses the already-tracked playlist's own ID and path** (submits the Add dialog again with identical values) instead of provisioning a second real YouTube resource — the path check is exercised without a second lookup/download.
- **Fixture identity is env-configurable**, not hardcoded: `SMOKE_PLAYLIST_ID`, `SMOKE_PLAYLIST_NAME`, `SMOKE_CHANNEL_HANDLE`, `SMOKE_CHANNEL_VIDEO_LIMIT`. The script forwards these to Playwright the same way it forwards `YOUTUBE_API_KEY`.
- **`scripts/run-smoke-tests.sh` performs the `docker build` itself** (not the workflow), so `.github/workflows/smoke-tests.yml` just installs Docker/Node and calls the script — local and CI runs take the identical path.
- **Readiness = polling `GET /status`**, not a fixed sleep, bounded by a timeout that fails the run with container logs if exceeded.
- **Cleanup via `trap ... EXIT`** in the script: on non-zero exit, dump `docker logs` to a file before `docker rm -f`; temp dirs always removed.

## Risks / Trade-offs

- [Real YouTube/yt-dlp dependency is inherently flaky (bot detection, quota, video/playlist removal)] → Accepted per proposal; retries (`retries: 2` on CI) and failure artifacts (logs, traces) make flakiness diagnosable, not silent.
- [Maintainer-owned playlist can drift out of sync with what tests expect] → Fixture IDs are env vars with one documented source of truth (this design + a comment in the workflow file), not scattered through test code.
- [`docker build` on every run is slow] → Accepted for v1; revisit with registry/layer caching only if it becomes a bottleneck.

## Migration Plan

New addition; nothing to migrate. Rollback is reverting the PR — no other code depends on `smoke-tests/` or the new workflow.

## Files

- `smoke-tests/package.json` — standalone Playwright project (kept separate from `web/`'s app dependencies).
- `smoke-tests/playwright.config.js` — base URL from `BASE_URL` env, single worker, CI retries, trace/video retained on failure.
- `smoke-tests/tests/playlist.spec.js` — Tier 1 playlist lifecycle: add → download → play → sync → duplicate-path error → delete.
- `smoke-tests/tests/channel.spec.js` — Tier 1 channel lifecycle: add → download → play → sync → invalid-handle error → delete.
- `smoke-tests/tests/mobile-sidebar.spec.js` — Tier 3 mobile sidebar open/close (no backend fixture needed).
- `smoke-tests/helpers/addDialog.js` — shared Add-dialog interactions (open, fill playlist form, fill channel form, read error).
- `smoke-tests/helpers/video.js` — shared polling assertions for download status and playback.
- `smoke-tests/helpers/sidebar.js` — shared sidebar sync/delete interactions.
- `scripts/run-smoke-tests.sh` — builds the image, runs the container, waits for readiness, runs Playwright, collects diagnostics on failure, always cleans up. Used both locally (sourcing repo-root `.env` for `YOUTUBE_API_KEY`, like `run-local.sh`) and by CI.
- `.github/workflows/smoke-tests.yml` — checks out the repo, sets up Docker/Node, runs the script with `secrets.YOUTUBE_API_KEY`, uploads `smoke-tests/test-results` and the saved container log as artifacts on failure.

## Types & Signatures

```js
// smoke-tests/helpers/addDialog.js
export async function openAddDialog(page) {}
export async function submitPlaylist(page, { url, name, path }) {}   // returns void; leaves dialog open on error
export async function submitChannel(page, { handle, videoLimit, path }) {}
export function dialogErrorText(page) {}                              // Locator for the visible error message

// smoke-tests/helpers/video.js
export async function waitForVideoStatus(page, { title, status, timeoutMs }) {}
export async function assertVideoPlays(page) {}                       // asserts the <video> element's readyState/duration

// smoke-tests/helpers/sidebar.js
export async function syncItem(page, { section, name }) {}            // section: 'Channels' | 'Playlists'
export async function deleteItem(page, { section, name }) {}
```

```bash
# scripts/run-smoke-tests.sh — env contract
YOUTUBE_API_KEY          # required; sourced from repo-root .env locally, from secrets in CI
SMOKE_PLAYLIST_ID        # required; the maintainer-owned "yarrtube-smoke-tests" playlist ID
SMOKE_PLAYLIST_NAME      # default: "yarrtube smoke tests"
SMOKE_CHANNEL_HANDLE     # default: "@BlenderOfficial"
SMOKE_CHANNEL_VIDEO_LIMIT # default: 1
YARRTUBE_PORT            # default: 8080
```

## Call Stack

**`scripts/run-smoke-tests.sh` (top level):**
```
main()
  docker build -t yarrtube:smoke .
  mktemp -d -> TMP_DIR (videos/, data/)
  docker run -d --name yarrtube-smoke \
      -p "$YARRTUBE_PORT:$YARRTUBE_PORT" \
      -e YOUTUBE_API_KEY -e YARRTUBE_PORT -e YARRTUBE_DB_PATH=/data/yarrtube.sqlite3 \
      -e YARRTUBE_VIDEOS_PATH=/videos \
      -v "$TMP_DIR/videos:/videos" -v "$TMP_DIR/data:/data" \
      yarrtube:smoke
  wait_for_status()            # curl -sf http://localhost:$YARRTUBE_PORT/status, loop w/ timeout
  npm --prefix smoke-tests ci
  npx --prefix smoke-tests playwright test    # BASE_URL, SMOKE_* forwarded via env
  trap cleanup EXIT            # docker logs > file (only if exit != 0); docker rm -f; rm -rf TMP_DIR
```

**`playlist.spec.js` (Tier 1):**
```
test('playlist lifecycle')
  openAddDialog(page)
  submitPlaylist(page, { url: SMOKE_PLAYLIST_ID, name: SMOKE_PLAYLIST_NAME })
  waitForVideoStatus(page, { title: <first video>, status: 'DOWNLOADED', timeoutMs: 120_000 })
  assertVideoPlays(page)
  syncItem(page, { section: 'Playlists', name: SMOKE_PLAYLIST_NAME })
  openAddDialog(page); submitPlaylist(page, { url: SMOKE_PLAYLIST_ID, name: SMOKE_PLAYLIST_NAME })
  expect(dialogErrorText(page)).toBeVisible()   // duplicate path
  deleteItem(page, { section: 'Playlists', name: SMOKE_PLAYLIST_NAME })
```

**`channel.spec.js` (Tier 1):**
```
test('channel lifecycle')
  openAddDialog(page)
  submitChannel(page, { handle: SMOKE_CHANNEL_HANDLE, videoLimit: SMOKE_CHANNEL_VIDEO_LIMIT })
  waitForVideoStatus(page, { status: 'DOWNLOADED', timeoutMs: 120_000 })   // no fixed title
  assertVideoPlays(page)
  syncItem(page, { section: 'Channels', name: SMOKE_CHANNEL_HANDLE })
  openAddDialog(page); submitChannel(page, { handle: 'this-handle-should-not-exist-abc123' })
  expect(dialogErrorText(page)).toBeVisible()   // invalid handle
  deleteItem(page, { section: 'Channels', name: SMOKE_CHANNEL_HANDLE })
```

**`mobile-sidebar.spec.js` (Tier 3, no fixture):**
```
test('mobile sidebar open/close')
  page.setViewportSize(mobile)
  page.click('[aria-label="Open menu"]')
  expect(sidebar).toBeVisible()
  page.click('[aria-label="Close menu"]')
  expect(sidebar).toBeHidden()
```

## Test Plan

- `playlist.spec.js :: playlist lifecycle` — asserts: playlist appears in sidebar after creation; its video reaches `DOWNLOADED`; the video renders in the playlist detail view and the home feed with thumbnail+duration; the `<video>` element loads and plays; re-submitting the same playlist shows a path-conflict error with Advanced options expanded; after deletion the playlist is gone from sidebar and home feed.
- `channel.spec.js :: channel lifecycle` — asserts: channel appears in sidebar; at least one video reaches `DOWNLOADED` (no title assumed); that video plays back; submitting a bogus handle shows an error; after deletion the channel is gone from sidebar and home feed.
- `mobile-sidebar.spec.js :: mobile sidebar open/close` — asserts: at a mobile viewport, the sidebar is hidden by default, becomes visible after tapping the menu button, and hides again after tapping its close control.
