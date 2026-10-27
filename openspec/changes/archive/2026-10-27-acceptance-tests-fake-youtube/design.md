## Files

- `src/serve.rs`: reads `YARRTUBE_YOUTUBE_API_URL` (default `https://www.googleapis.com/youtube/v3`), `YARRTUBE_YTDLP_SELF_UPDATE` (`false` turns it off; anything else, or unset, leaves it on) and `YARRTUBE_BACKGROUND_POLL_INTERVAL_MS` (default 5000; the task executor's and event consumer's poll), skips the startup update when off, and passes them on.
- `src/infrastructure/infrastructure_container.rs`: `InfrastructureSettings` gains `youtube_api_url`, given to the four Data API adapters.
- `src/infrastructure/repositories/youtube_playlist_repository.rs`, `youtube_playlist_items_repository.rs`, `youtube_metadata_repository.rs`, `youtube_channel_repository.rs`: `new(api_key, api_url)` takes the API root and appends its own path (`/playlists`, `/playlistItems` + `/videos`, `/videos`, `/channels`); the per-file endpoint constants and test-only `with_base_url` go.
- `src/application/tasks/update_ytdlp_task.rs`: `UpdateYtdlpTask` takes whether the self-update is on; when off, a run skips the update and still reschedules.
- `doc/INSTALLATION.md`: the two settings, marked as for testing against a fake YouTube.
- `smoke-tests/fake-youtube/catalogue.json` (new): the fixture world: videos (title, duration, publish time, behaviour: `normal` / `slow` / `failing`, `upcoming`), playlists and the channel.
- `smoke-tests/fake-youtube/youtube-stub.js` (new): the fake Data API on the host, like `plex-stub.js`; also serves the fixture images (thumbnails, avatar).
- `smoke-tests/fake-youtube/yt-dlp` (new): the stand-in `yt-dlp`, a Deno script (the image ships Deno for `yt-dlp`) answering the daemon's six invocations from the catalogue and `smoke-tests/samples/`.
- `smoke-tests/samples/`: the CC clip `c-dQKjG5ffc.mp4` (61 s, 240p H.264/AAC), its `.jpg`, `CREDITS.txt`, and `fixture.en.srt` (our own subtitles).
- `smoke-tests/helpers/fakeYoutube.js` (new): the catalogue's playlists, channel and videos for specs to use by name.
- `smoke-tests/helpers/seed.js`: `addPlaylist` / `addChannel` add a fixture source through the API and optionally wait for its downloads; the shared `beforeAll` seeding goes.
- `smoke-tests/helpers/fixtures.js`: rewrites the stub's container-side host (`host.docker.internal`) to `localhost` for the browser, so fixture avatars load in the Add channel dialog.
- `smoke-tests/tests/*.spec.js`: every spec using `SMOKE_PLAYLIST_*`, `SMOKE_CHANNEL_*` or the seeded downloads moves to the fixture sources; `media.spec.js` loses the codec test.
- `smoke-tests/tests/erroredDownload.spec.js`, `search.spec.js`, `premiere.spec.js` (new): the new coverage.
- `scripts/run-smoke-tests.sh`: starts the YouTube stub, mounts `smoke-tests/fake-youtube/` and `smoke-tests/samples/` read-only, configures the container (`YTDLP_PATH`, `YARRTUBE_YOUTUBE_API_URL`, `YARRTUBE_YTDLP_SELF_UPDATE=false`, a dummy `YOUTUBE_API_KEY`, `YARRTUBE_RETRY_BASE_DELAY_SECONDS=0` (a failing fixture download takes 2 s per attempt), `YARRTUBE_BACKGROUND_POLL_INTERVAL_MS=250`), and runs the upgrade check only with a real `YOUTUBE_API_KEY`.
- `smoke-tests/README.md`: the fake YouTube, its catalogue, and what still needs the real one.

## Types & Signatures

```rust
// src/serve.rs
const DEFAULT_YOUTUBE_API_URL: &str = "https://www.googleapis.com/youtube/v3";
fn youtube_api_url() -> String;                                   // YARRTUBE_YOUTUBE_API_URL
fn ytdlp_self_update_enabled() -> bool;                           // YARRTUBE_YTDLP_SELF_UPDATE
fn parse_ytdlp_self_update(value: Option<String>) -> bool;        // only "false" turns it off
fn background_poll_interval() -> Duration;                        // YARRTUBE_BACKGROUND_POLL_INTERVAL_MS
fn parse_background_poll_interval(value: Option<String>) -> Duration; // positive ms, else 5 s

// src/infrastructure/infrastructure_container.rs
pub struct InfrastructureSettings { /* … */ pub youtube_api_url: String }

// the four Data API adapters
impl YoutubeApiPlaylistRepository      { pub fn new(api_key: String, api_url: String) -> Self }
impl YoutubeApiPlaylistItemsRepository { pub fn new(api_key: String, api_url: String) -> Self }
impl YoutubeApiMetadataRepository      { pub fn new(api_key: String, api_url: String) -> Self }
impl YoutubeApiChannelRepository       { pub fn new(api_key: String, api_url: String) -> Self }

// src/application/tasks/update_ytdlp_task.rs
impl UpdateYtdlpTask {
    pub fn new(updater, ytdlp_path, self_update_enabled: bool, task_repository, clock) -> Self
}
```

```js
// smoke-tests/helpers/fakeYoutube.js
export const PLAYLIST          // { id, name, path, videos: [{ id, title }] }: the golden-path playlist
export const SLOW_PLAYLIST     // its video downloads for ~20 s
export const BROKEN_PLAYLIST   // its video's download always fails
export const PREMIERE_PLAYLIST // an upcoming premiere + a regular video
export const SEARCH_PLAYLISTS  // 16 small playlists for the search specs
export const CHANNEL           // { handle, name, path, videos: [newest, older] }

// smoke-tests/helpers/seed.js
export async function addPlaylist(request, playlist, { downloaded = true } = {}) // → the created playlist
export async function addChannel(request, channel, { videoLimit, downloaded = true })
```

## Call Stack

- Run: `run-smoke-tests.sh` → `node smoke-tests/fake-youtube/youtube-stub.js` (host port) → `docker run … -v fake-youtube:/fake-youtube:ro -v samples:/samples:ro -e YTDLP_PATH=/fake-youtube/yt-dlp -e YARRTUBE_YOUTUBE_API_URL=http://host.docker.internal:<port>/youtube/v3 -e YARRTUBE_YTDLP_SELF_UPDATE=false -e YARRTUBE_BACKGROUND_POLL_INTERVAL_MS=250` → Playwright.
- Data API: adapter → `GET <api_url>/playlists?part=snippet,contentDetails&id=` | `/playlistItems?part=snippet,status&maxResults=50&playlistId=` | `/videos?part=snippet&id=` | `/channels?part=id,snippet&forHandle=` → stub → catalogue → the JSON fields each adapter reads (thumbnail and avatar URLs point back at the stub).
- yt-dlp: daemon → `/fake-youtube/yt-dlp <args> <url>` → video id from the URL (or the info JSON for `--load-info-json`) → per mode: download copies the sample `.mp4` and `.jpg` as `<id>.*` and prints timestamp, upload date, duration, filename (`slow` sleeps first, `failing` prints an `ERROR:` and exits 1); thumbnail-only copies the `.jpg`; `-J` prints the info JSON (`en` uploaded subtitles); `--load-info-json` writes `<id>.en.srt`; `--simulate` exits 0 (`failing`: the error); `--flat-playlist` prints the channel's videos newest first, as JSON lines.
- Spec: `addPlaylist(request, PLAYLIST)` → `POST /api/playlists` → poll `/api/playlists/{id}/videos` until downloaded (seconds) → the test drives the UI.

## Test Plan

1. Behaviour tests (Rust, application layer):
   1. `it_should_skip_the_update_and_reschedule_if_self_update_is_off` (`update_ytdlp_task`): the updater is not called, and the next run is scheduled an interval later.
2. Infrastructure tests:
   - `serve.rs` settings:
     1. `it_should_read_the_youtube_api_url_or_default_it`: unset → the default; set → that URL.
     2. `it_should_turn_the_ytdlp_self_update_off_only_if_false`: `None`, `"true"`, `"abc"` → on; `"false"` → off.
     3. `it_should_read_the_background_poll_interval_or_default_it`: `None`, `"0"`, `"-1"`, `"abc"` → 5 s; `"250"` → 250 ms.
   - Data API adapters: their existing mock-server tests move to `new(api_key, server.url())` and match the endpoint path (`/playlists`, `/playlistItems`, `/videos`, `/channels`), proving each appends its path to the configured root.
3. Smoke (Playwright, against the fake YouTube), each run with `scripts/run-smoke-tests.sh`:
   1. The stand-in and stub: `playlist golden path` passes on the fixture playlist (download in seconds, on disk, Plex scan, badge, delete).
   2. Every spec moved to the fixture sources, file by file, passes: `playlist`, `channel`, `channelVideoLimit`, `moveSources`, `categories`, `addDialogLocation`, `addPlaylistDialog`, `addChannelDialog`, `deepLinks`, `deleteDuringDownload`, `media` (Range only), `playbackSpeed`, `mobile-sidebar`, `mobileWatch`, `mobileLayout`.
   3. `erroredDownload.spec.js`: `a download that keeps failing is shown as errored with its reason`.
   4. `search.spec.js`: `sidebar search lists only matching playlists`, `tasks search lists only matching tasks`.
   5. `premiere.spec.js`: `an upcoming premiere is neither listed nor downloaded`.
