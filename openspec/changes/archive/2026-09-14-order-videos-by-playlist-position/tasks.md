## 1. Capture position from YouTube

- [x] 1.1 Add `position: i64` to `PlaylistItemSnippet` (`#[serde(rename = "position")]`) and to `PlaylistVideo` in `src/infrastructure/repositories/youtube_playlist_items_repository.rs`, and populate it in `fetch_page`'s mapping. Verify with a test asserting `list_current_videos` returns the `position` value from a mocked API response.
- [x] 1.2 Update `FakeYoutubePlaylistItemsRepository` and any existing constructors/fixtures of `PlaylistVideo` across the codebase to supply a position, and verify `cargo build --release` succeeds with no leftover call sites.

## 2. Persist position on the video

- [x] 2.1 Add `pub position: Option<i64>` to `Video` (`src/domain/video/video.rs`), defaulting to `None` in `Video::create`'s existing call sites that don't yet know a position, and add/adjust a constructor helper so a YouTube-linked video can be created with a known position. Verify with a unit test that a created video carries the expected position.
- [x] 2.2 Add a nullable `position INTEGER` column to the `videos` table in `SqliteVideoRepository::new`'s `CREATE TABLE IF NOT EXISTS`, and read/write it in `row_to_video`, `save`, and `update`. Verify with a round-trip test: save a video with a position, read it back, assert the position matches; save one with no position, read it back, assert it's `None`.

## 3. Track position through reconcile

- [x] 3.1 In `VideoService::sync_playlist_membership` (`src/domain/video/service.rs`), pass the fetched `PlaylistVideo.position` into both the `Video::create` branch (new video) and the `Video { ..stored }` rebuild branch (existing video), so position is set on first discovery and refreshed on every subsequent pass. Verify with a unit test that reconciling a playlist twice, with a changed position for an already-stored video between passes, updates the stored position after the second pass.
- [x] 3.2 Verify `add_video_to_custom_playlist` (`src/domain/video/service.rs`) continues to create videos with no position (`None`), confirming custom-playlist videos are unaffected — add/adjust a test asserting this explicitly if none already covers it.

## 4. Order the listing

- [x] 4.1 Change `SqliteVideoRepository::list_for_playlist`'s query to `ORDER BY position IS NULL, position ASC, video_id ASC`. Verify with a test seeding videos with out-of-insertion-order positions for one playlist and asserting `list_for_playlist` returns them in position order, plus a test mixing positioned and NULL-position rows and asserting NULLs sort last, in `video_id ASC` order among themselves.
- [x] 4.2 Add/adjust the HTTP-level test in `src/http/videos/mod.rs` to assert `GET /playlists/{id}/videos` returns videos ordered by playlist position for a YouTube-linked playlist.

## 5. Verify end to end

- [x] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo build --release`, and `cargo test --locked`, and confirm all pass.
