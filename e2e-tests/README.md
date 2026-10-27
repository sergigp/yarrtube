# End-to-end tests

A Playwright suite that drives the real Docker image through a browser
against a fake YouTube: adding playlists and channels, "downloading" their
videos and playing them back, with the daemon's real database, task queue
and file layout. YouTube's own side (the real `yt-dlp` and Data API) is
covered by the YouTube contract tests (see `doc/DEVELOPMENT.md`). Only the
upgrade check still uses the real YouTube.

Run it with:

```bash
./scripts/run-e2e-tests.sh
```

from the repository root. It builds the release Docker image, runs it with
a fresh temp database/videos directory, waits for it to become ready, then
runs this Playwright suite against it. Before that, it runs the upgrade
check (see below) unless `E2E_UPGRADE_FROM_IMAGE=none`. The container and temp directories
are always removed afterward; on failure, the container's logs are saved to
`e2e-tests/container.log` and Playwright's traces/videos are kept under
`e2e-tests/test-results/`.

## The fake YouTube

The daemon in the container is pointed at a fake YouTube that serves one
catalogue, `fake-youtube/catalogue.json`: playlists, a channel and their
videos. Specs use them by name through `helpers/fakeYoutube.js`.

- `fake-youtube/youtube-stub.js`: the YouTube Data API (playlists, their
  items, videos, channels by handle) plus the thumbnail and avatar images,
  run on the host. The container reaches it through
  `YARRTUBE_YOUTUBE_API_URL`; the browser through `localhost` (the page
  fixture maps the host).
- `fake-youtube/yt-dlp`: a stand-in `yt-dlp` (a Deno script, the image ships
  Deno) mounted into the container as `YTDLP_PATH`. It answers each way the
  daemon calls `yt-dlp` the way the contract tests show the real one does.
  Every video is the same Creative-Commons clip, `samples/c-dQKjG5ffc.mp4`
  (credits in `samples/CREDITS.txt`), with our own subtitles
  (`samples/fixture.en.srt`). A `slow` video takes ~8 s to download, a
  `failing` one always fails (2 s into each attempt), and an `upcoming` one is
  a premiere.

The container also runs with the self-update off
(`YARRTUBE_YTDLP_SELF_UPDATE=false`, it would replace the stand-in), a 250 ms
background poll (`YARRTUBE_BACKGROUND_POLL_INTERVAL_MS`, so each pipeline step
doesn't wait 5 s) and no delay between retries.

Overrides (defaults shown):

| Env var                      | Default                 |
| ----------------------------- | ------------------------ |
| `YARRTUBE_PORT`               | `8080`                   |
| `E2E_READY_TIMEOUT_SECONDS` | `120`                    |
| `E2E_PLEX_STUB_PORT`        | `18324`                  |
| `E2E_YOUTUBE_STUB_PORT`     | `18326`                  |
| `E2E_UPGRADE_FROM_IMAGE`    | `ghcr.io/sergigp/yarrtube:latest` (`none` skips the upgrade check) |

The upgrade check starts the released image, which can't be pointed at the
fake YouTube, so it needs a real `YOUTUBE_API_KEY` (from the environment or
the repository's `.env`) and is skipped without one. It uses the
maintainer-owned "test" playlist (`E2E_PLAYLIST_ID`, default
`PLXWRoRTUXjks`, holding Big Buck Bunny) and retries failed downloads after
`E2E_RETRY_BASE_DELAY_SECONDS` (default `10`).

## Browsers

Two Playwright projects run against the same container:

- **`chromium`** (Desktop Chrome) runs every spec *not* tagged `@mobile`.
- **`mobile-safari`** (`iPhone 13`: WebKit, touch, mobile viewport and user
  agent) runs only the specs tagged `@mobile` (a `@mobile` suffix in the
  test's title). Phone layouts are tested here, never by resizing the desktop
  browser. There is no Android/Chromium phone project yet, to keep run time
  down.

## What every test gets

Specs import `test`/`expect` from `helpers/fixtures.js`, not
`@playwright/test`. Each test starts from an empty library (leftovers from
an earlier failed test are deleted through the API), unless it opts out
with `test.use({ keepLibrary: true })`, and fails on:

- an uncaught page error;
- a `console.error` not matched by the test's `consoleErrorAllowlist`
  (failed resource loads are left to the request checks, and WebKit's own
  video-control icon errors are ignored);
- a failed or ≥400 `/media` or `/avatars` request (requests the browser
  cancels itself, e.g. while seeking, don't count);
- a 5xx `/api` response, or a 4xx one the test didn't declare in
  `expectedApiErrors` (`[{ method, path, status }]`, `path` a pathname or a
  RegExp).

Tests add the sources they need themselves with `helpers/seed.js`
(`addPlaylist` / `addChannel`: added through the API and, by default, waited
on until downloaded and settled, about a second), so no test depends on
another's state. Image assertions use
`expectImageLoaded` (`helpers/media.js`), which requires the image to have
actually loaded rather than the placeholder a broken one is swapped for.

## What's covered

- **Upgrade** (`upgrade/`): the released image (`E2E_UPGRADE_FROM_IMAGE`)
  adds the fixture playlist through the API, downloads it and records a
  watch position (`seed.spec.js`); it is stopped and this build starts on
  the same database and videos (`verify.spec.js`), which must list the
  playlist with its settings, keep the video downloaded (nothing queued to
  download again) with its position, play it from the playlist's URL and
  keep its next sync scheduled. This is also the suite's restart check.

- **Playlist** (`tests/playlist.spec.js`): one golden path adds the fixture
  playlist via the Add dialog, waits for its download, checks it on disk as
  a Plex episode (`Season 1/S01E…` media, `.nfo`, thumbnail, `tvshow.nfo`)
  and that Plex was asked to scan its folder, sees its unwatched badge
  arrive through the sidebar's periodic refresh (no reload), then deletes it
  and checks its folder is removed. On a downloaded playlist, independent tests
  hide and show its badge, list it with a loaded thumbnail and duration,
  play it and record the position, show its subtitles, show it on the home
  feed (its card showing the time left), show its progress bar and time left
  in the playlist's list, exclude it from home and include it again, sync it (changing
  nothing), edit its check interval, run its sync from the Tasks view,
  reject adding it twice, mark it watched while playing and mark it all
  watched. The Tasks view's Downloads tab lists the slow video's download
  while it runs.
- **Channel** (`tests/channel.spec.js`): the same lifecycle for the fixture
  channel (episodes under `Season <year>`, plus a `poster.jpg`), including
  marking a single video watched, editing its quality and check interval.
  While the video is unwatched, the folded Channels heading shows
  the section's unwatched total and is marked active on the channel's
  route. Separately, the channel's avatar in the sidebar (the copy the
  daemon downloaded) loads.
- **Channel video limit** (`tests/channelVideoLimit.spec.js`): a channel
  with two downloaded videos, edited down to one: the next sync drops the
  older video from the list and its files from disk, keeping the newer.
- **Moving sources** (`tests/moveSources.spec.js`): once the fixture
  playlist, then the fixture channel, has downloaded and settled, its edit
  dialog shows its Location and "Move…" renames its show folder: the
  episode is on disk at the new path and gone from the old, Plex is asked
  to scan both folders, and the video still plays.
- **Delete during download** (`tests/deleteDuringDownload.spec.js`):
  deleting a playlist while its download is running removes its folder, and
  it doesn't reappear once the download ends.
- **Media** (`tests/media.spec.js`): `/media` answers
  `Range: bytes=0-1023` with a `206`, a `Content-Range` and 1024 bytes.
  Every episode checked on disk must be `.mp4` (that real downloads are
  H.264/AAC is checked by the YouTube contract tests).
- **Errored download** (`tests/erroredDownload.spec.js`): a download that
  keeps failing is listed in the Downloads tab with its reason while it's
  retried, and its video is Errored once the retries run out.
- **Premieres** (`tests/premiere.spec.js`): a playlist's upcoming premiere
  is neither listed nor downloaded; its other video is.
- **Search** (`tests/search.spec.js`): with 16 playlists, the sidebar search
  and the Tasks view's search list only what matches.
- **Playback speed** (`tests/playbackSpeed.spec.js`): choosing 1.5x from the
  speed menu sets the player's `playbackRate` and the menu shows it.
- **Deep links** (`tests/deepLinks.spec.js`): reloading home, a playlist, a
  channel, a category and the Tasks view shows that view again, and an
  untracked playlist's route says "Playlist not found.".
- **Add dialogs** (`tests/addPlaylistDialog.spec.js`,
  `tests/addChannelDialog.spec.js`, `tests/addDialogLocation.spec.js`): the
  destination notice (title, count, absolute path), values that aren't a
  playlist or channel, playlists and channels YouTube doesn't know, the
  channel's avatar in the notice,
  auto-filled folder names, and choosing, staging and blocking destinations
  with the folder browser.
- **Categories** (`tests/categories.spec.js`): create a category from the
  sidebar (landing in its manage sources dialog), rename it in place,
  assign the fixture channel to it from the Add channel dialog, remove it
  and add it back with the source chips, mark it all watched once the video
  downloads, fold the section and delete the category from its actions
  menu, keeping the channel. A category's card opens its video, selected,
  in the channel's view and plays it. On the iPhone, the "Add category" row
  closes the drawer before its dialog opens, a new category's manage
  sources dialog fills the screen, and once a category exists the
  heading's "+" adds the next one.
- **Mobile drawer** (`tests/mobile-sidebar.spec.js`, iPhone): the menu
  toggle opens and closes the drawer (its `aria-expanded` follows), as do
  the backdrop and Escape; it stays closable when an announcement appears
  after it opened; tapping an entry navigates and closes it; while it is
  open the page behind doesn't scroll (PageDown, as mobile WebKit has no
  mouse wheel), and closing it keeps the scroll position.
- **Mobile watching** (`tests/mobileWatch.spec.js`, iPhone): a home card
  opens its video, selected, which plays in WebKit and resumes after a
  reload; the player sticks under the header while the page scrolls; video
  details start collapsed and "More" expands them.
- **Mobile layout** (`tests/mobileLayout.spec.js`, iPhone): home, playlist,
  channel, category and Tasks views never scroll sideways; the Add (with
  the folder browser open), Edit settings and Delete dialogs fit the screen
  and submit; the icon-only Sync, Tasks tabs and Run now work by their
  accessible names.
- **Sidebar folding** (`tests/sidebarFolding.spec.js`): a folded section
  stays folded across a reload while the other keeps its own state, on
  desktop and in the iPhone drawer.
- **Tasks view** (`tests/tasks.spec.js`): the recurring yt-dlp self-update
  task the daemon seeds at startup is listed as pending.
- **Announcements** (`tests/announcements.spec.js`): the bar shows, links,
  dismisses and remembers dismissals, and stays hidden when the fetch fails.
- **Version** (`tests/version.spec.js`): the header shows the running version.

Together these are also the only coverage of the HTTP API's route wiring
(Rust handler tests call handlers directly, bypassing the router), so every
`/api` route must be exercised by at least one spec. This is enforced: the
browser's `/api` responses are recorded with their status, and
`routeCoverageReporter.js` fails a passing full run
(`E2E_CHECK_ROUTE_COVERAGE=1`, set by `run-e2e-tests.sh`) if any route
parsed from `src/application/http/mod.rs` was never answered with a 2xx.

After the suite, `run-e2e-tests.sh` also fails the run if the container
logged a panic.

### Not covered yet

- A full run against the real YouTube: the YouTube contract tests cover
  YouTube's side, and only the upgrade check downloads real videos.
- On a real iPhone (checked by hand): native captions menu, fullscreen and
  rotation, sound, and the home-screen web app.

## Local development

```bash
npm --prefix e2e-tests ci
npx --prefix e2e-tests playwright install --with-deps chromium webkit
```

Then start the stubs (`node e2e-tests/plex-stub.js`,
`node e2e-tests/fake-youtube/youtube-stub.js`) and a yarrtube container
configured like `scripts/run-e2e-tests.sh`'s `start_container ... fake`
(fake YouTube URL, stand-in `yt-dlp` and samples mounted, self-update off),
and run:

```bash
BASE_URL=http://localhost:8080 \
  E2E_VIDEOS_DIR=$PWD/videos \
  E2E_PLEX_STUB_URL=http://localhost:18324 \
  E2E_YOUTUBE_STUB_PORT=18326 \
  npm --prefix e2e-tests test
```

Add `--project=chromium` or `--project=mobile-safari` to run one browser
only.
