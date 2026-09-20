## 1. Fixture prerequisites

- [x] 1.1 Create the public "yarrtube-smoke-tests" YouTube playlist (containing the Creative-Commons "Big Buck Bunny" video) and record its playlist ID; document it as the required `SMOKE_PLAYLIST_ID` value in `smoke-tests/README.md` and `.github/workflows/smoke-tests.yml`
- [x] 1.2 Add `YOUTUBE_API_KEY` as a GitHub Actions repo secret and verify it's referenced (not printed) in the new workflow file

## 2. Playwright project scaffold

- [x] 2.1 Create `smoke-tests/package.json` (Playwright as the only dependency) and `smoke-tests/playwright.config.js` (base URL from `BASE_URL` env, `workers: 1`, `fullyParallel: false`, CI retries, trace/video `retain-on-failure`); verify `npm --prefix smoke-tests ci` succeeds
- [x] 2.2 Verify `npx --prefix smoke-tests playwright install --with-deps chromium` succeeds locally

## 3. Shared helpers

- [x] 3.1 Implement `smoke-tests/helpers/addDialog.js` (`openAddDialog`, `submitPlaylist`, `submitChannel`, `dialogErrorText`) per design.md's Types & Signatures
- [x] 3.2 Implement `smoke-tests/helpers/video.js` (`waitForVideoStatus`, `assertVideoPlays`)
- [x] 3.3 Implement `smoke-tests/helpers/sidebar.js` (`syncItem`, `deleteItem`)

## 4. Playlist lifecycle test

- [x] 4.1 Write `smoke-tests/tests/playlist.spec.js` covering: add via Add dialog, video reaches `DOWNLOADED`, appears in playlist detail + home feed with thumbnail/duration, plays back — satisfies spec.md's "Playlist Golden Path"
- [x] 4.2 Extend the same test with the sidebar Sync action and the duplicate-path error assertion (Advanced options auto-expands) — satisfies "Sync Action Coverage" and the duplicate-path scenario of "Add-Dialog Error Handling Coverage"
- [x] 4.3 Extend the same test with deletion via the sidebar and assert it disappears from sidebar + home feed — completes "Playlist Golden Path"

## 5. Channel lifecycle test

- [x] 5.1 Write `smoke-tests/tests/channel.spec.js` covering: add `@BlenderOfficial` with `video_limit: 1`, at least one video reaches `DOWNLOADED` (no fixed title asserted), plays back — satisfies "Channel Golden Path"
- [x] 5.2 Extend with the sidebar Sync action and the invalid-handle error assertion — satisfies "Sync Action Coverage" and the invalid-handle scenario of "Add-Dialog Error Handling Coverage"
- [x] 5.3 Extend with deletion via the sidebar and assert it disappears from sidebar + home feed — completes "Channel Golden Path"

## 6. Mobile sidebar test

- [x] 6.1 Write `smoke-tests/tests/mobile-sidebar.spec.js`: at a mobile viewport, open the sidebar via the menu button and close it via its close control/overlay — satisfies "Mobile Sidebar Coverage"

## 7. Driver script

- [x] 7.1 Write `scripts/run-smoke-tests.sh`: source repo-root `.env` if present, validate required env vars, `docker build`, verify the image builds successfully
- [x] 7.2 Add container run + `wait_for_status` polling of `GET /status` with a bounded timeout; verify the script exits non-zero with container logs printed when readiness is never reached (e.g. by pointing at a bad image tag)
- [ ] 7.3 Add the Playwright invocation, forwarding `BASE_URL` and `SMOKE_*`/`YOUTUBE_API_KEY` env vars; verify a full local run (`./scripts/run-smoke-tests.sh`) passes end to end — **blocked on this machine**: a pre-existing bug in `src/infrastructure/client/ytdlp_updater.rs` (`asset_name_for_platform` ignores CPU architecture) makes the startup self-update overwrite the image's correct arm64 `yt-dlp` with an x86_64 build that can't run under Docker Desktop's Rosetta layer, so every download fails on this arm64 Mac. Verified the full script logic and all three specs pass end to end with that one binary swapped for a correct arm64 copy; unaffected on amd64 CI runners. See chat for details.
- [x] 7.4 Add failure diagnostics (save `docker logs` to a file) and a `trap ... EXIT` cleanup that always removes the container and temp directories; verify by forcing a test failure and confirming the container/temp dirs are gone afterward and the log file exists

## 8. CI workflow

- [x] 8.1 Add `.github/workflows/smoke-tests.yml` triggered on `pull_request` and `push` to `main`, running `scripts/run-smoke-tests.sh` with `secrets.YOUTUBE_API_KEY` and the documented `SMOKE_*` fixture values
- [ ] 8.2 Add `actions/upload-artifact` steps (conditioned on failure) for `smoke-tests/test-results` and the saved container log; verify by triggering the workflow via `workflow_dispatch` and confirming a successful run, then confirming artifacts are attached on an intentionally forced failure
