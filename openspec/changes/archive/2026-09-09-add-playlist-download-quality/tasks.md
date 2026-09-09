## 1. Domain: Quality value object and Playlist field

- [x] 1.1 Add a `Quality` value object under `domain/playlist/` (mirroring `PlaylistName`'s pattern: `new`/parse from `"high"`/`"mid"`/`"low"`, `Err` on anything else) and unit-test valid/invalid parsing
- [x] 1.2 Add `quality: Quality` to `Playlist` (`domain/playlist/playlist.rs`), update `Playlist::create` to take it, and update its existing unit test to assert the field
- [x] 1.3 Add/update `domain/playlist/errors.rs` for an invalid-quality error variant and confirm it renders a meaningful message (unit test)

## 2. Storage: persist quality

- [x] 2.1 Add a `quality` column to the `playlists` table DDL in `sqlite_playlist_repository.rs` and map it on insert/read; update `FakePlaylistRepository` (test double) to carry `quality` too
- [x] 2.2 Update every repository test/fixture across the codebase that builds a `Playlist::create(...)` to pass a quality, and run `cargo test --locked` to confirm nothing is left uncompiling

## 3. HTTP: accept and return quality

- [x] 3.1 Add `quality: String` to `CreatePlaylistRequest` and `quality` to `PlaylistResponse` (`http/playlists/dto.rs`), parsing/validating it into `Quality` the same way the handler validates `name` today
- [x] 3.2 Update the create-playlist handler (`http/playlists/mod.rs`) to reject a missing/invalid `quality` with a bad request before any YouTube API call, matching the invalid-name behavior; add handler tests for: valid quality, missing quality, invalid quality value, and duplicate-ID-with-different-quality (existing record's quality is kept)
- [x] 3.3 Update the list-playlists handler test to assert `quality` is present in each returned playlist

## 4. yt-dlp: quality-to-args mapping

- [x] 4.1 In `infrastructure/shared/ytdlp.rs`, add a function mapping `Quality` to yt-dlp args per design.md's selector table (`high`/`mid`/`low` → `-f`/`-S`/`--merge-output-format`) and unit-test each tier's exact arg list
- [x] 4.2 Change `download_video` to accept the resolved args and pass them to the `yt-dlp` `Command` before the URL; update its existing tests (fake yt-dlp binary) to cover a call with args

## 5. Task payload and download plumbing

- [x] 5.1 Add `quality: String` to `Task::DownloadVideo`, update `Task::payload()` and `decode_download_video_payload` (now returns `(playlist_id, video_id, quality)`), and update `task.rs`'s existing round-trip/malformed-payload tests
- [x] 5.2 Give `DownloadVideoOnVideoAdded` a `PlaylistRepository` dependency; in `handle`, look up the playlist, resolve `quality`, and include it when scheduling `Task::DownloadVideo`; update its tests (including a case where the playlist no longer exists — no-op, matching the existing "video no longer exists" pattern) and its call site in `subscribers/mod.rs`
- [x] 5.3 Change `VideoDownloaderRepository::download` to accept `Quality` (or the resolved args) alongside `video_url`/`output_dir`, threading it to `ytdlp::download_video`; update `YtDlpVideoDownloaderRepository` and `FakeVideoDownloaderRepository`
- [x] 5.4 Change `VideoService::download_video` to accept `quality` as a parameter (decoded from the task payload) and pass it to `video_downloader_repository.download`, instead of reading it off the freshly-looked-up `Playlist`; update its call site in `DownloadVideoTask::handle` and all of `service.rs`'s existing tests

## 6. Remove the standalone CLI download command

- [x] 6.1 Delete `cli/download_command.rs`, the `Commands::Download` variant (`cli/mod.rs` or wherever `Cli`/`Commands` is defined), and its dispatch arm in `main.rs`
- [x] 6.2 Delete `infrastructure/client/youtube_downloader_client.rs` (`YoutubeDownloaderClient`/`YtDlpDownloaderClient`) and its module wiring
- [x] 6.3 Remove the now-dead `download_all`/`DownloadSummary` machinery if nothing else references it; run `cargo build --release` to confirm no leftover references
- [x] 6.4 Update `README.md`: drop the `yarrtube download` line from "Manual commands" and add `quality` to the `POST /playlists` row's example payload

## 7. Full verification

- [x] 7.1 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` all pass
- [ ] 7.2 Manually create a playlist via `POST /playlists` with each of `high`/`mid`/`low`, let a video download, and confirm (via `ffprobe` or similar) the resulting file is mp4/h264/aac and respects the tier's resolution cap
