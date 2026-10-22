# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end to end, with stub bodies only:
  - `lib/watchProgress.ts`: `watchProgress` returns `null`, and `formatTimeLeft` returns `''`.
  - `ThumbnailOverlay` renders today's tick and duration badge (no bar, since `watchProgress` is `null`).
  - `WatchProgressBar` takes `{ percent, size }`, and `WatchedTick` takes `{ watched, size, raised }`.
  - `VideoGrid` and `VideoListPane` render `ThumbnailOverlay`.
  - `showProgress` is removed from `VideoGrid` and `Home`.
  - No dimming yet.

  Done when: `npm run check` passes. The existing Home test that counts one progress bar under "Continue watching" is the only expected break. Mark it `it.todo` with a pointer to 2.10, which replaces it. No new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `watchProgress.test.ts`, `returns progress for an unwatched started video`.
- [x] 2.2 `watchProgress.test.ts`, `returns null for an unstarted, watched or durationless video`.
- [x] 2.3 `watchProgress.test.ts`, `clamps a position past the duration`.
- [x] 2.4 `watchProgress.test.ts`, `formats time left in minutes`.
- [x] 2.5 `watchProgress.test.ts`, `formats time left under a minute`.
- [x] 2.6 `watchProgress.test.ts`, `formats time left from one hour`.
- [x] 2.7 `ChannelDetail.test.tsx`, `shows progress and time left on an in-progress row`: drives the bar and time-left badge in `ThumbnailOverlay` (row size).
- [x] 2.8 `PlaylistDetail.test.tsx`, `shows progress and time left on an in-progress row`.
- [x] 2.9 `CategoryDetail.test.tsx`, `shows progress and time left on an in-progress card`: card size.
- [x] 2.10 `Home.test.tsx`, `shows progress on in-progress cards in every section`: replaces the `it.todo` from 1.1.
- [x] 2.11 `Home.test.tsx`, `shows no progress for watched or durationless cards`.
- [x] 2.12 Styling, no test (see design.md Test Plan note):
  - Bar: `h-1.5` on cards, `h-1` on rows, `bg-black/50` track.
  - The tick and badge are raised above the bar when it's shown.
  - Watched: thumbnail `opacity-60` (back to full on card `group-hover`) and `text-muted-foreground` title, with the active row keeping `text-primary`.

  Existing suite stays green.

## 3. Infrastructure adapters (TDD)

None: no backend change.

## 4. Verification

- [x] 4.1 `npm run check` passes in `web/`. `cargo build --release` embeds the new dist, and `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` still pass.
- [x] 4.2 Smoke: in `smoke-tests/tests/playlist.spec.js`:
  - The home card assertion expects the time left, since the played video now shows that instead of its duration.
  - A new step checks the playlist's video list shows the progress bar and time left.

  Then `scripts/run-smoke-tests.sh` passes (existing `Watched` tick assertions in `categories.spec.js` must still hold).
- [x] 4.3 Manual (agent):
  1. Start `scripts/run-local.sh --port <free port> --fresh-videos`.
  2. Through the HTTP API, add a channel, wait for a few downloads, then use `POST /api/videos/:id/progress` to put one video at ~30% and mark another watched.
  3. With the Playwright MCP browser, check:
     - The home grid, the category view and the channel list each show the bar plus `Nm left` on the in-progress video, and nothing on the others.
     - The watched video's thumbnail has computed opacity < 1 and its title has the muted color. Hovering a watched home card brings the opacity back to 1.
     - Selecting the watched row keeps its title in the primary color.
  4. Take screenshots at desktop and 390px width.
  5. Stop the server, and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human):
  - Visual judgement on desktop and phone width: is the bar thick and contrasted enough on bright and dark thumbnails?
  - The tick, badge and bar don't collide on 96px list thumbnails.
  - Watched dimming reads as "done" without hiding the video.
