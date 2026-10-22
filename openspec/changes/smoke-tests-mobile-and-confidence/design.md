## Files

- `smoke-tests/playwright.config.js`: add a `mobile-safari` project (`devices['iPhone 13']`, `grep: /@mobile/`); the desktop `chromium` project gets `grepInvert: /@mobile/` (the drawer specs can't run at desktop width).
- `scripts/run-smoke-tests.sh`: `playwright install --with-deps chromium webkit`; export `SMOKE_CONTAINER_NAME`.
- `smoke-tests/helpers/fixtures.js`: console-error / requestfailed / unexpected-4xx checks, `expectedApiErrors` and `consoleErrorAllowlist` options.
- `smoke-tests/helpers/seed.js` (new): `seedDownloadedPlaylist` / `seedDownloadedChannel` (API-created, low quality, polled to DOWNLOADED) for a file's `beforeAll`, so specs sharing one download are independent tests.
- `smoke-tests/helpers/routeCoverage.js`: record status; `hitRoutes()` returns only routes answered 2xx.
- `smoke-tests/helpers/media.js` (new): `ffprobe` via `docker exec`, Range request check, `expectImageLoaded`.
- `smoke-tests/helpers/library.js`: `MEDIA_EXTENSIONS = ['mp4']`.
- `smoke-tests/helpers/sidebar.js`: role-based `sectionList` (`getByRole('list', { name })`), new `drawer` helpers (`openDrawer`, `closeDrawer`, `expectDrawerOpen/Closed` via toggle `aria-expanded` + `#app-sidebar` in viewport).
- `smoke-tests/helpers/addDialog.js`: `dialogErrorText` → `getByRole('alert')`.
- `smoke-tests/helpers/layout.js` (new): `expectNoHorizontalOverflow`, `expectWithinViewport`.
- `smoke-tests/tests/mobile-sidebar.spec.js`: `@mobile`, no `setViewportSize`, drawer helpers, + drawer navigation/Escape/scroll tests.
- `smoke-tests/tests/mobileWatch.spec.js` (new, `@mobile`): home card → play → resume; sticky player; collapsed details; WebKit playback.
- `smoke-tests/tests/mobileLayout.spec.js` (new, `@mobile`): no overflow per route; dialogs fit; icon-only controls.
- `smoke-tests/tests/media.spec.js` (new): codec + Range checks on `seedDownloadedPlaylist`.
- `smoke-tests/tests/playbackSpeed.spec.js` (new): speed menu changes `playbackRate`.
- `smoke-tests/tests/deepLinks.spec.js` (new): reload every route; unknown playlist → "Playlist not found.".
- `smoke-tests/tests/deleteDuringDownload.spec.js` (new): delete mid-download leaves no folder.
- `smoke-tests/tests/playlist.spec.js`: split into independent tests on `seedDownloadedPlaylist`; add Downloads tab (fresh add); one badge check without reload.
- `smoke-tests/tests/categories.spec.js`: drop `setViewportSize` test into `@mobile`; add category card → plays.
- `smoke-tests/tests/channel.spec.js`: remove duplicated invalid-handle step; avatar loaded check.
- `smoke-tests/tests/addPlaylistDialog.spec.js`: unknown playlist id test.
- `smoke-tests/tests/sidebarFolding.spec.js`: mobile test → `@mobile`, drawer helpers.
- `smoke-tests/tests/announcements.spec.js`: replace `networkidle` with awaiting the announcements response.
- `smoke-tests/README.md`: projects, `@mobile` tag, new coverage, out-of-scope list.
- `web/src/components/Sidebar.tsx`, `SidebarSectionHeading.tsx`: heading gets an `id`, `<ul aria-labelledby>` (Categories already has `aria-label`).
- `web/src/components/AddChannelDialog.tsx`, `AddPlaylistDialog.tsx`: submit error `<p role="alert">`.
- `web/src/components/Sidebar.test.tsx`, `AddPlaylistDialog.test.tsx`, `AddChannelDialog.test.tsx`: Vitest coverage for the two a11y changes.

## Types & Signatures

```js
// helpers/fixtures.js
test.extend({
  expectedApiErrors: [[], { option: true }],   // e.g. [{ method: 'POST', path: '/api/playlists', status: 409 }]
  consoleErrorAllowlist: [[], { option: true }], // RegExp[]
})

// helpers/seed.js — used in a spec file's beforeAll/afterAll with test.use({ keepLibrary: true })
export async function seedDownloadedPlaylist(request)         // → { id, name, path, video }
export async function seedDownloadedChannel(request, { videoLimit }) // → { handle, name, path, videos }
export async function deleteSeeded(request, seeded)

// helpers/media.js
export async function expectH264Mp4(relativePath)            // docker exec $SMOKE_CONTAINER_NAME ffprobe …
export async function expectRangeSupported(request, mediaUrl) // 206, Content-Range, 1024 bytes
export async function expectImageLoaded(locator)              // complete && naturalWidth > 0

// helpers/layout.js
export async function expectNoHorizontalOverflow(page)        // scrollWidth <= innerWidth
export async function expectWithinViewport(page, locator)     // bbox inside viewportSize()

// helpers/sidebar.js
export function sectionList(page, section)                    // getByRole('list', { name: section })
export async function openDrawer(page)
export async function closeDrawer(page)
export async function expectDrawerOpen(page)                  // toggle aria-expanded=true, #app-sidebar in viewport
export async function expectDrawerClosed(page)

// helpers/routeCoverage.js
export function recordApiRoute(method, pathname, status)
export function hitRoutes()                                   // Set of routes with ≥1 2xx
```

## Call Stack

- Project selection: `playwright test` → project `mobile-safari` → `grep /@mobile/` → spec `test('… @mobile', …)`.
- Health check: `page` fixture → `page.on('console'|'requestfailed'|'response')` → push problem unless allowlisted / in `expectedApiErrors` → after `use(page)`: `expect(problems).toEqual([])`.
- Route coverage: `response` handler → `recordApiRoute(method, pathname, status)` → append `METHOD /tpl STATUS` → reporter `onEnd` → `hitRoutes()` (2xx only) vs `routerRoutes()`.
- Shared download: spec `test.beforeAll` → `seedDownloadedPlaylist(request)` → `deleteAllSources` → `request.post('/api/playlists', { playlist, path, quality: 'low', exclude_from_home: false, reconcile_interval_minutes: 180 })` → poll `/api/playlists/{id}/videos` until all `DOWNLOADED` with thumbnails → tests run with `keepLibrary: true` → `test.afterAll` → `deleteSeeded` + wait for folder removal.
- Codec: `expectH264Mp4(path)` → `execFile('docker', ['exec', SMOKE_CONTAINER_NAME, 'ffprobe', '-v', 'error', '-show_entries', 'stream=codec_name,codec_type', '-of', 'json', '/videos/' + path])`.
- Watch flow: `page.goto('/')` → tap card `getByRole('link')` with title → `toHaveURL(/\?video=/)` → `main h3` = title → `assertVideoPlays` → set `currentTime`, pause → await `/progress` → reload → `currentTime > 0`.

## Test Plan

These are Playwright specs against existing behaviour, so most should pass on first run. A spec that passes straight away is kept as a regression test, per the apply guidance. A spec that fails shows a real bug or a wrong assumption: stop and report it.

1. Suite behaviour (smoke specs), simplest first:
   1. `mobile sidebar opens and closes via toggle and backdrop @mobile`: `aria-expanded` follows the drawer state; backdrop tap closes.
   2. `drawer closes on Escape @mobile`
   3. `tapping a drawer entry navigates and closes it @mobile`
   4. `dismissing the drawer keeps the scroll position and the page behind stays still @mobile`
   5. `no view scrolls horizontally @mobile`: `/`, playlist, channel, category, `/tasks`.
   6. `home card opens the video selected, plays and resumes after reload @mobile`
   7. `player sticks under the header while the list scrolls @mobile`
   8. `video details start collapsed and More expands them @mobile`
   9. `add, edit-settings and delete dialogs fit the screen and submit @mobile`
   10. `icon-only Tasks tabs and Run now work by accessible name @mobile`
   11. `category card opens its video in the channel view and plays`
   12. `speed menu sets playbackRate to 1.5`
   13. `reloading each route shows its view; unknown playlist says Playlist not found.`
   14. `unknown playlist id is reported in the Add playlist dialog`
   15. `Downloads tab lists a running download`
   16. `downloaded video is h264/aac mp4`
   17. `media answers a Range request with 206`
   18. `channel avatar loads`
   19. `deleting a playlist mid-download leaves no folder`
   20. `unwatched badge appears via periodic refresh without reload`
2. Suite infrastructure (helpers and fixtures). Each check is proven by a throwaway failing case, then reverted:
   1. `console.error not allowlisted fails the test`
   2. `unexpected 4xx /api fails the test; declared one passes`
   3. `failed /media request fails the test`
   4. `route answered only 4xx is reported uncovered`
   5. `expectImageLoaded fails on a broken src`
   6. `expectEpisodeOnDisk rejects a .webm`
3. Web UI (Vitest, `web/`):
   1. `it should expose each sidebar section as a list named after its heading`
   2. `it should show the add playlist submit error as an alert`
   3. `it should show the add channel submit error as an alert`

Risk: Playwright's WebKit build may lack H.264 decoding on some hosts. If WebKit can't play the h264 file, keep the ffprobe codec check, mark the WebKit playback assertion `test.fixme` with a comment, and leave it to the manual iPhone check.
