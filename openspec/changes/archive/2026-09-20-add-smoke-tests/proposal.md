## Why

Unit and integration tests exercise individual layers, but nothing today proves that a built Docker image actually serves a working webapp end to end: create a playlist/channel, download a real video via yt-dlp, and play it back. That gap matters most exactly when it's riskiest to verify by hand — large refactors across layers — so we want an automated, CI-enforced smoke suite that drives the real Docker image through a browser (Playwright) and can also be run locally on demand before/after a big refactor.

## What Changes

- Add a new top-level `smoke-tests/` directory: a Playwright test project (own `package.json`/config) covering the golden-path flows below against a running Docker container.
- Add `scripts/run-smoke-tests.sh`: builds the release Docker image from the repo's `Dockerfile`, runs it with a real `YOUTUBE_API_KEY` and fresh temp DB/videos paths, polls `GET /status` for readiness, runs the Playwright suite against it, dumps container logs and Playwright traces on failure, and always tears the container and temp dirs down.
- Add a new GitHub Actions workflow that runs this script on every PR and push to `main`, using a repo secret `YOUTUBE_API_KEY`.
- Flows covered (real YouTube Data API + real yt-dlp downloads, no test doubles):
  - Add a `youtube_linked` playlist (a maintainer-owned public "yarrtube-smoke-tests" playlist containing the Creative-Commons "Big Buck Bunny" video) via the Add dialog, watch it download, appear in the playlist detail view and the home feed, play back in the browser, then delete it via the sidebar.
  - Add a `youtube_linked` channel (`@BlenderOfficial`, `video_limit: 1`) via the Add dialog and repeat the same download/playback/delete flow with loose (non-title) assertions.
  - Sidebar "Sync" (reconcile) button on an already-tracked playlist/channel.
  - Add-dialog error paths: duplicate path conflict, invalid channel handle.
  - Mobile sidebar open/close at a narrow viewport.
- Out of scope for this change: custom playlists (no UI entry point yet; README lists them as "Coming soon").

## Capabilities

### New Capabilities
- `smoke-tests`: an automated Playwright suite plus a driver script that builds the Docker image, runs it, and exercises the webapp's golden paths (playlist/channel creation, real download pipeline, playback, deletion) and a handful of cheap secondary flows (sync, add-dialog errors, mobile sidebar), run locally on demand and in CI on every PR/push to `main`.

### Modified Capabilities
(none — this only adds a new testing capability; no existing product requirements change)

## Impact

- New files only: `smoke-tests/**`, `scripts/run-smoke-tests.sh`, `.github/workflows/smoke-tests.yml`. No changes to `src/` or `web/src/` application code.
- New CI dependency: a `YOUTUBE_API_KEY` GitHub Actions repo secret (maintainer-provided) and network egress to `googleapis.com`/`youtube.com` from CI runners — accepted tradeoff for genuine end-to-end coverage; flakiness from YouTube-side bot detection or quota will be addressed if/when it occurs.
- New maintainer-owned fixture: a public YouTube playlist ("yarrtube-smoke-tests") that must keep containing the expected video(s).
- Slower CI: every PR/push now also builds a Docker image and runs a real download, on top of the existing `fmt`/`clippy`/`build`/`test` job.
