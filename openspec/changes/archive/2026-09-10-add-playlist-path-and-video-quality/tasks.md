## 1. Relocate `Quality` to `domain/shared/`

- [x] 1.1 Move `src/domain/playlist/quality.rs` to `src/domain/shared/quality.rs` unchanged (enum, `new`, `as_str`, `Display`, existing tests), and update `src/domain/shared/mod.rs`/`src/domain/playlist/mod.rs` exports accordingly; verify `cargo build` succeeds
- [x] 1.2 Update every `use crate::domain::playlist::Quality` (and re-exports) across the codebase — `domain/video/service.rs`, `domain/task/task.rs`, `infrastructure/shared/ytdlp.rs`, `infrastructure/repositories/youtube_video_downloader_repository.rs`, `http/playlists/dto.rs`, and their test modules — to `use crate::domain::shared::Quality`; verify `cargo build --release` and `cargo test --locked` pass with no behavior change

## 2. `PlaylistPath` value object

- [x] 2.1 Add `src/domain/playlist/playlist_path.rs` with `PlaylistPath::new`/`as_str`/`Display`, rejecting: empty (or all-whitespace) input, a leading `/` (absolute path), any `..` segment, any empty segment (`//`, leading/trailing `/`), and the same filesystem-unsafe characters `PlaylistName` rejects per segment; verify unit tests cover each rejection case plus a valid multi-segment path (e.g. `a/b/c`) and a valid single-segment path
- [x] 2.2 Export `PlaylistPath` from `src/domain/playlist/mod.rs`; verify `cargo build` succeeds

## 3. `Playlist.path`

- [x] 3.1 Add `path: PlaylistPath` to `Playlist` (`src/domain/playlist/playlist.rs`) and thread it through `Playlist::create`; update its unit test to cover the new field; verify `cargo test playlist::` passes
- [x] 3.2 Add a `path TEXT NOT NULL` column to the `playlists` table in `sqlite_playlist_repository.rs`'s `CREATE TABLE IF NOT EXISTS`, and thread `path` through `insert`, `find`, `list`, and `row_to_playlist`; update `FakePlaylistRepository` test helpers; verify `cargo test sqlite_playlist_repository::` passes
- [x] 3.3 Add required `path: String` to `CreatePlaylistRequest` and `path: String` to `PlaylistResponse` (`src/http/playlists/dto.rs`); verify serialization/deserialization unit or doc tests cover the new field
- [x] 3.4 Wire `path` through the create-playlist HTTP handler and `PlaylistService::create_playlist` (parse `PlaylistPath::new`, reject invalid input the same way invalid `name`/`quality` are rejected today, without checking YouTube first); verify handler tests cover: successful creation with a nested path, missing path, invalid path (absolute, `..`, empty segment), and duplicate playlist ID with a different path (ignored, original path returned)

## 4. Storage location now uses `path`, not `name`

- [x] 4.1 In `VideoService::download_video` and `VideoService::delete_video_file` (`src/domain/video/service.rs`), build `output_dir` from `playlist.path.as_str()` instead of `playlist.name.as_str()`; verify existing download/delete service tests pass with an updated fixture playlist path, plus a new test asserting a multi-segment path produces the expected nested `output_dir`

## 5. `Video.quality`

- [x] 5.1 Add `quality: Option<Quality>` to `Video` (`src/domain/video/video.rs`); change `mark_downloaded` to take the resolved `Quality` and set it; update existing `mark_downloaded` unit tests to pass a quality and assert it's recorded, and confirm other transition methods (`start_download`, `mark_errored_retrying`, `mark_errored`) leave `quality` untouched
- [x] 5.2 Add a nullable `quality TEXT` column to the `videos` table in `sqlite_video_repository.rs`'s `CREATE TABLE IF NOT EXISTS`, and thread it through `save`, `find`, `list_for_playlist`, `update`, and `row_to_video` (`NULL` until a video has a recorded quality); update `FakeVideoRepository` test helpers; verify `cargo test sqlite_video_repository::` passes, including a round-trip test for a video with no recorded quality and one with a recorded quality
- [x] 5.3 Update `VideoService::download_video` to call `mark_downloaded` with the `quality: Quality` parameter it already receives; verify a service-level test asserts the video's `quality` is set after a successful download and remains `None` after a failed one

## 6. Full verification

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo build --release`, and `cargo test --locked`; verify all pass
- [x] 6.2 Manually exercise create-playlist with a nested `path` (e.g. `a/b/c`) against a running `serve` instance and confirm a downloaded video lands at `<YARRTUBE_VIDEOS_PATH>/a/b/c/<file>`
