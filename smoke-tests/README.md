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

Specs that share one download seed it in `beforeAll` with
`helpers/seed.js` (`seedDownloadedPlaylist` / `seedDownloadedChannel`: added
through the API at low quality and waited on until downloaded and settled),
so each of their tests runs on its own. Image assertions use
`expectImageLoaded` (`helpers/media.js`), which requires the image to have
actually loaded rather than the placeholder a broken one is swapped for.

## What's covered

- **Upgrade** (`upgrade/`): the released image (`SMOKE_UPGRADE_FROM_IMAGE`)
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
  and checks its folder is removed. On a seeded download, independent tests
  hide and show its badge, list it with a loaded thumbnail and duration,
  play it and record the position, show its subtitles, show it on the home
  feed, exclude it from home and include it again, sync it (changing
  nothing), edit its check interval, run its sync from the Tasks view,
  reject adding it twice, mark it watched while playing and mark it all
  watched. On a fresh add, the Tasks view's Downloads tab lists its
  download while it runs.
- **Channel** (`tests/channel.spec.js`): the same lifecycle for the fixture
  channel (episodes under `Season <year>`, plus a `poster.jpg`), including
  marking a single video watched, editing its quality and check interval,
  with loose (non-title) assertions since the channel's newest video can
  change. While the video is unwatched, the folded Channels heading shows
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
- **Media** (`tests/media.spec.js`): the downloaded video is an `.mp4` with
  H.264 video and AAC audio (read with `ffprobe` inside the container
  named by `SMOKE_CONTAINER_NAME`), and `/media` answers
  `Range: bytes=0-1023` with a `206`, a `Content-Range` and 1024 bytes.
  Every episode checked on disk must be `.mp4`.
- **Playback speed** (`tests/playbackSpeed.spec.js`): choosing 1.5x from the
  speed menu sets the player's `playbackRate` and the menu shows it.
- **Deep links** (`tests/deepLinks.spec.js`): reloading home, a playlist, a
  channel, a category and the Tasks view shows that view again, and an
  untracked playlist's route says "Playlist not found.".
- **Add dialogs** (`tests/addPlaylistDialog.spec.js`,
  `tests/addChannelDialog.spec.js`, `tests/addDialogLocation.spec.js`): the
  destination notice (title, count, absolute path), values that aren't a
  playlist or channel, playlists and channels YouTube doesn't know,
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
(`SMOKE_CHECK_ROUTE_COVERAGE=1`, set by `run-smoke-tests.sh`) if any route
parsed from `src/application/http/mod.rs` was never answered with a 2xx.

After the suite, `run-smoke-tests.sh` also fails the run if the container
logged a panic.

### Not covered yet

- An errored download being surfaced: it needs a maintainer-owned playlist
  holding an unavailable video.
- Sidebar search and Tasks search: they only show with more than 15 sources
  or tasks, more than the fixtures track.
- On a real iPhone (checked by hand): native captions menu, fullscreen and
  rotation, sound, and the home-screen web app.

## Local development

```bash
npm --prefix smoke-tests ci
npx --prefix smoke-tests playwright install --with-deps chromium webkit
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
  SMOKE_CONTAINER_NAME=<the container, for ffprobe> \
  npm --prefix smoke-tests test
```

`SMOKE_CONTAINER_NAME` names the Docker container serving `BASE_URL`; only
`tests/media.spec.js` uses it, to run `ffprobe` on the downloaded file, so
against a `cargo run` instance that spec can't pass. Add
`--project=chromium` or `--project=mobile-safari` to run one browser only.
