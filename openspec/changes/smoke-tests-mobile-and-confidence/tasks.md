## 1. Walking skeleton

- [x] 1.1 Create every file and signature from design.md with trivial bodies, wired end-to-end:
  - the `mobile-safari` project with `grep: /@mobile/` and `grepInvert: /@mobile/` on `chromium`
  - webkit install and `SMOKE_CONTAINER_NAME` in `run-smoke-tests.sh`
  - new helpers (`media.js`, `layout.js`, `seed.js`, the drawer helpers) whose bodies return or do nothing
  - the new fixture options `expectedApiErrors` and `consoleErrorAllowlist`, accepted but not enforced
  - new spec files with empty `test.fixme` placeholders
  - the existing mobile specs retagged `@mobile`, with `setViewportSize` removed

  Done when `npx --prefix smoke-tests playwright test --list` lists the `@mobile` specs only under `mobile-safari` and the untagged specs only under `chromium`, and `npm --prefix web run check` passes.

## 2. Behaviour (TDD)

Each smoke spec: write it, run it against `scripts/run-local.sh` or the smoke image, and expect it to pass (existing behaviour). A spec that fails points to a real bug or a wrong assumption: stop and report it.

- [x] 2.1 `mobile sidebar opens and closes via toggle and backdrop @mobile`: rewrite `mobile-sidebar.spec.js` with the drawer helpers (`aria-expanded` plus `#app-sidebar` in the viewport); passes on `mobile-safari`.
- [x] 2.2 `drawer closes on Escape @mobile`: passes on `mobile-safari`.
- [x] 2.3 `tapping a drawer entry navigates and closes it @mobile`: seed a playlist via the API, tap it in the drawer, URL is `/playlists/…` and the drawer is closed.
- [x] 2.4 `dismissing the drawer keeps the scroll position and the page behind stays still @mobile`: PageDown (mobile WebKit has no mouse wheel) scrolls the page while closed but leaves `scrollY` unchanged while the drawer is open, and closing keeps it.
- [x] 2.5 `no view scrolls horizontally @mobile`: `expectNoHorizontalOverflow` on `/`, playlist, channel, category and `/tasks` with a seeded library.
- [x] 2.6 `home card opens the video selected, plays and resumes after reload @mobile`: uses `seedDownloadedPlaylist`.
- [x] 2.7 `player sticks under the header while the list scrolls @mobile`: the player's top equals the header's bottom after scrolling `main`.
- [x] 2.8 `video details start collapsed and More expands them @mobile`: "Open on YouTube" is hidden, then visible after tapping More.
- [ ] 2.9 `add, edit-settings and delete dialogs fit the screen and submit @mobile`: `expectWithinViewport` on each dialog and its submit button; each submits successfully.
- [ ] 2.10 `icon-only Tasks tabs and Run now work by accessible name @mobile`: select Syncs by name, Run now posts `/reconcile` with a 2xx response.
- [ ] 2.11 Move the mobile category test in `categories.spec.js` to `@mobile` (no `setViewportSize`, dialog box compared with `viewportSize()`); passes on `mobile-safari`.
- [ ] 2.12 Retag the mobile test in `sidebarFolding.spec.js` `@mobile` and use the drawer helpers; passes on `mobile-safari`.
- [ ] 2.13 `category card opens its video in the channel view and plays`: add it to `categories.spec.js`; URL is `/channels/…?video=…`, title matches, `assertVideoPlays`.
- [ ] 2.14 `speed menu sets playbackRate to 1.5`: `playbackSpeed.spec.js` on `seedDownloadedPlaylist`; `el.playbackRate === 1.5` and the menu shows 1.5×.
- [ ] 2.15 `reloading each route shows its view; unknown playlist says Playlist not found.`: `deepLinks.spec.js`.
- [ ] 2.16 `unknown playlist id is reported in the Add playlist dialog`: `addPlaylistDialog.spec.js`; the notice contains "does not exist", has `role`/tone error, and Create is disabled.
- [ ] 2.17 `playlist quality edit is kept`: Edit settings → Low → reopen shows Low.
- [ ] 2.18 `Downloads tab lists a running download`: add the playlist via the API at high quality, then the Downloads tab lists `Downloading …` before it finishes.
- [ ] 2.19 `downloaded video is h264/aac mp4`: `media.spec.js`, `expectH264Mp4` on the seeded video.
- [ ] 2.20 `media answers a Range request with 206`: `media.spec.js`, `expectRangeSupported` on `/media/<path>/<filename>`.
- [ ] 2.21 `channel avatar loads`: in `channel.spec.js`, `expectImageLoaded` on the sidebar avatar and the notice avatar; also remove the duplicated invalid-handle step.
- [ ] 2.22 `deleting a playlist mid-download leaves no folder`: `deleteDuringDownload.spec.js`; delete while a download task is running, then `expectGoneFromDisk` stays gone after waiting out the task.
- [ ] 2.23 `unwatched badge appears via periodic refresh without reload`: in the playlist specs; poll the badge within the sidebar refresh interval, no `page.reload()`.
- [ ] 2.24 Split `playlist.spec.js` into independent tests on `seedDownloadedPlaylist`, keeping one UI-add golden path. Every former step still exists as a test and the file passes.
- [ ] 2.25 Replace `networkidle` in `announcements.spec.js` with awaiting the announcements response; the spec passes.

## 3. Infrastructure adapters (TDD)

Suite helpers: prove each check with a throwaway failing case, then revert it.

- [ ] 3.1 `console.error not allowlisted fails the test`: enforce `consoleErrorAllowlist` in `fixtures.js`. Prove it with a temporary `page.evaluate(() => console.error('x'))`.
- [ ] 3.2 `unexpected 4xx /api fails the test; declared one passes`: enforce `expectedApiErrors`. Declare the expected errors on every existing spec that triggers 4xx (duplicate, invalid, preview misses).
- [ ] 3.3 `failed /media request fails the test`: `requestfailed` and ≥400 responses for `/media` and `/avatars`. Prove it with a request to a missing media path.
- [ ] 3.4 `route answered only 4xx is reported uncovered`: `recordApiRoute(…, status)`, and `hitRoutes()` returns only routes with a 2xx. Prove it with a hit file holding only a 4xx line.
- [ ] 3.5 `expectImageLoaded fails on a broken src`: implement it and use it in every thumbnail assertion (playlist detail list, home card, category card).
- [ ] 3.6 `expectEpisodeOnDisk rejects a .webm`: set `MEDIA_EXTENSIONS = ['mp4']`. Prove it on a temporary dir.
- [ ] 3.7 Switch `sectionList`/`sectionRows`/`sectionEmptyMessage` in `helpers/sidebar.js` to role-based lookups once 4.2 lands. All sidebar-using specs pass.
- [ ] 3.8 Make `dialogErrorText` in `helpers/addDialog.js` use `getByRole('alert')` once 4.3 lands. Replace the `toHaveClass(/text-destructive/)` assertions where an accessible equivalent exists. The specs pass.

## 4. Web UI a11y (Vitest TDD, `web/`)

- [x] 4.1 `it should expose each sidebar section as a list named after its heading`: give the heading an `id` and the `<ul>` `aria-labelledby` in `Sidebar.tsx`/`SidebarSectionHeading.tsx`; `npm --prefix web run check` passes.
- [x] 4.2 `it should show the add playlist submit error as an alert`: `role="alert"` in `AddPlaylistDialog.tsx`; check passes.
- [x] 4.3 `it should show the add channel submit error as an alert`: `role="alert"` in `AddChannelDialog.tsx`; check passes.

## 5. Verification

- [ ] 5.1 `npm --prefix web run check`, `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 5.2 `./scripts/run-smoke-tests.sh` passes end to end on both projects (`chromium`, `mobile-safari`), including the upgrade check and route coverage (2xx-only). Record the total run time.
- [ ] 5.3 Update `smoke-tests/README.md`: projects and the `@mobile` tag, new coverage, health checks, and the out-of-scope list (errored-download fixture, Sidebar/Tasks search). The README matches the spec files.
- [ ] 5.4 Manual (human), on a real iPhone (Safari, plus the home-screen web app):
  - open home, tap a card, the video plays with sound, and it resumes after a reload
  - turn captions on from the native player menu
  - fullscreen and rotate
  - open and close the drawer, and navigate from it
