## 1. Video duration: capture from `yt-dlp`

- [x] 1.1 In `src/infrastructure/shared/ytdlp.rs`, add a `--print` directive for duration (e.g. `%(duration)s`) *before* the existing `--print after_move:filename` in `download_video`'s args, so the filename stays the last stdout line. Parse the new second-to-last line into an `Option<i64>` (whole seconds; treat unparsable/empty as `None` rather than failing the download). Verify with unit tests: duration present and parsed, duration missing/unparsable yields `None` without affecting filename parsing, existing filename-parsing tests still pass.
- [x] 1.2 Update `download_video`'s return type (and its one caller) to carry the optional duration alongside the filename. Verify it compiles and existing callers are updated.

## 2. Video duration: domain and persistence

- [x] 2.1 Add `duration_seconds: Option<i64>` to `Video` (`src/domain/video/video.rs`); thread it through `mark_downloaded` (new parameter) and clear it in `reset_for_redownload`, mirroring `thumbnail_filename`. Verify with unit tests mirroring the existing thumbnail ones: recorded when present, `None` when absent, cleared on reset.
- [x] 2.2 Update `VideoDownloader::download` (`src/domain/services/video_downloader.rs`) to pass the parsed duration from task 1.2 into `mark_downloaded`. Verify with unit tests covering both a duration present and a duration absent.
- [x] 2.3 Add a `duration_seconds INTEGER` column to the `videos` table's `CREATE TABLE IF NOT EXISTS` in `src/infrastructure/repositories/sqlite_video_repository.rs`, and read/write it in `save`/`find`/`update`. Verify with repository unit tests round-tripping a video with and without a recorded duration.

## 3. Video duration: HTTP and SPA

- [x] 3.1 Add `duration_seconds: Option<i64>` to `VideoResponse` and `RecentVideoResponse` (`src/application/http/videos/dto.rs`), populated from `Video`/`RecentVideo`. Verify with unit tests on both `From` conversions: duration present, duration absent (serializes as `null`/omitted consistent with `thumbnail_filename`'s existing handling).
- [x] 3.2 Add a small duration-formatting helper in the SPA (e.g. `formatDuration(seconds)` → `"12:34"` / `"1:02:03"`) and render it as a badge/label on each video card or row in `Home.jsx`, `PlaylistDetail.jsx`, and `ChannelDetail.jsx` when `duration_seconds` is present; render nothing (no placeholder needed, unlike thumbnails) when absent. Verify by checking the rendered markup for a video with a duration and one without, in each component.

## 4. Channel avatar: resolve the URL from YouTube

- [x] 4.1 In `src/infrastructure/repositories/youtube_channel_repository.rs`, extend `ChannelSnippet` to deserialize `thumbnails` (`default`/`medium`/`high`, each with a `url`), add `avatar_url: Option<String>` to `ResolvedChannel`, and populate it from `thumbnails.medium.url` falling back to `thumbnails.default.url`. Verify with unit tests: avatar URL present, thumbnails absent from the response entirely (`avatar_url` is `None`).

## 5. Channel avatar: download, store, and serve

- [x] 5.1 Add a new port (e.g. `ChannelAvatarRepository`) with a method like `store(handle: &ChannelHandle, url: &str) -> anyhow::Result<Option<String>>` that downloads the image bytes via `reqwest` and writes them to a local avatars directory as `<handle>.<ext>` (extension from content type, defaulting to `jpg`), returning the stored filename. A network/write failure returns `Ok(None)` rather than an error (mirrors the thumbnail-unavailable precedent), reserving `Err` for a systemic problem. Add a matching `delete(filename: &str)` method. Verify with unit tests against a mock HTTP server: successful download, HTTP error status, unreachable host — all resulting in `Ok(None)` except the successful case.
- [x] 5.2 Wire a real implementation's avatars root path (configurable or a sensible default like `./avatars`, analogous to `DEFAULT_DB_PATH` in `src/serve.rs`) and mount it at `/avatars` via a second `tower_http::services::ServeDir` in `src/serve.rs`, parallel to the existing `/media` mount. Verify with a router test analogous to the existing `/media` tests: a known file under the avatars root is served, and path traversal outside it is rejected.

## 6. Channel avatar: domain and persistence

- [x] 6.1 Add `avatar_filename: Option<String>` to `Channel` (`src/domain/channel/channel.rs`) and its `create` constructor. Verify with a unit test building a channel with and without an avatar filename.
- [x] 6.2 Add an `avatar_filename TEXT` column to the `channels` table's `CREATE TABLE IF NOT EXISTS` in `src/infrastructure/repositories/sqlite_channel_repository.rs`, and read/write it in `find`/`insert`/`list`. Verify with repository unit tests round-tripping a channel with and without a recorded avatar filename.
- [x] 6.3 Inject the `ChannelAvatarRepository` port into `ChannelService`. In `create_channel`, after a successful YouTube resolve, call `store` with the resolved `avatar_url` (when present) and record the result on the created `Channel`; any `None`/failure leaves `avatar_filename` unset without failing creation. In `delete_channel`, call `delete` with the channel's recorded avatar filename when present, before or alongside the existing DB deletion. Verify with unit tests (extending the existing `ChannelService` test suite and its fakes): avatar recorded on successful store, no avatar recorded when the port returns `None`, avatar deletion invoked on channel deletion when a filename was recorded, no deletion invoked when none was recorded.
- [x] 6.4 Wire the real `ChannelAvatarRepository` implementation into `serve.rs`'s composition root alongside the other `ChannelService` dependencies.

## 7. Channel avatar: HTTP and SPA

- [x] 7.1 Add `avatar_filename: Option<String>` to `ChannelResponse` (`src/application/http/channels/dto.rs`). Verify with a unit test on `From<Channel>` covering both presence and absence.
- [x] 7.2 Thread the channel's `avatar_filename` through `VideoSource::Channel`/`RecentVideoSourceResponse` (`src/domain/video/recent_video.rs`, `src/domain/services/video_searcher.rs`, `src/application/http/videos/dto.rs`) the same zero-extra-query way `path` was threaded through in the thumbnails-to-listings change — `VideoSearcher::list_recent` already loads the full `Channel` per source. `VideoSource::Playlist` sources have no avatar. Verify with unit tests: a channel source's avatar filename is included when present and absent when not, a playlist source never includes one.
- [x] 7.3 Add an `avatarMediaUrl(filename)` helper to `web/src/api.js` (building a `/avatars/...` URL, analogous to `videoMediaUrl`).
- [x] 7.4 Render the channel avatar (a small circular image, falling back to the existing `Thumbnail.jsx` placeholder pattern when absent) in: `Sidebar.jsx`'s channel list rows, `Home.jsx`'s video cards for channel-sourced videos, and next to the video title in `ChannelDetail.jsx`'s `VideoDetail`. Add supporting styles to `web/src/App.css`. Verify by checking rendered markup for a channel with an avatar and one without, in each location.

## 8. Verification

- [x] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; all must pass.
- [x] 8.2 Run `npm run lint` and `npm run build` in `web/`; both must pass.
- [x] 8.3 Manually exercise the full flow against a running daemon: create a channel and confirm its avatar appears in the sidebar and (once it has downloaded videos) on its video cards and detail view; download a video and confirm its duration renders on its card/row in Home, its playlist, and its channel view; delete the channel and confirm its avatar file is removed from the local avatars directory.
