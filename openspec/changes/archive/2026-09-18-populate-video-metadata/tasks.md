## 1. Dependencies and domain entity

- [x] 1.1 Add an XML-serialization crate (e.g. `quick-xml`) to `Cargo.toml` and verify `cargo build` succeeds
- [x] 1.2 Add `src/domain/video_metadata/video_metadata.rs` with the `VideoMetadata` entity (title, plot, studio, director, premiered, year, genre, tags, uniqueid, thumb, sorttitle) and verify it compiles with unit tests for its constructor/builder

## 2. YouTube metadata fetching

- [x] 2.1 Add `YoutubeMetadata` struct and `YoutubeMetadataRepository` trait (`find(&VideoId) -> Result<Option<YoutubeMetadata>>`) in `src/infrastructure/repositories/youtube_metadata_repository.rs`, parsing `title`, `description`, `channelTitle`, `publishedAt`, `tags`, `categoryId` from `videos.list?part=snippet`, and verify with a mockito-backed test mirroring `youtube_video_repository.rs`'s existing tests
- [x] 2.2 Add a `FakeYoutubeMetadataRepository` test double in the same module, following the existing `FakeYoutubeVideoRepository` pattern
- [x] 2.3 Add the hardcoded YouTube `categoryId` → genre-name mapping function and verify unit tests cover a mapped ID, an unmapped ID (returns `None`), and a missing `categoryId`

## 3. Metadata mapping and formatting

- [x] 3.1 Implement plot truncation (500 chars, cut at last word boundary, trailing ellipsis when truncated) as a small pure function and verify unit tests cover: under limit unchanged, over limit truncated at a word boundary, exactly-at-limit unchanged
- [x] 3.2 Implement `sorttitle` resolution: zero-padded `PlaylistVideo.position` when present; otherwise a zero-padded prefix from `YoutubeMetadata.published_at`, using `PlaylistVideoRepository::find_by_video`/`ChannelVideoRepository::find_by_video` to determine which applies, and verify unit tests cover a playlist video with a position, a custom-playlist video (no position), and a channel video (position present but ignored)
- [x] 3.3 Implement the `YoutubeMetadata` + resolved `sorttitle` + thumbnail filename → `VideoMetadata` assembly function and verify unit tests cover the with-thumbnail and without-thumbnail cases
- [x] 3.4 Implement `movie.nfo` XML rendering from `VideoMetadata` using the XML crate and verify unit tests cover: all fields present, thumbnail absent, no tags, unmapped genre omitted, and a title/description containing `&`, `<`, `>`, `"`, `'` round-trips as valid, correctly-escaped XML

## 4. VideoMetadataRepository (composed persistence)

- [x] 4.1 Add the `video_metadata` SQLite table (embedded `CREATE TABLE IF NOT EXISTS`, matching this repo's existing per-repository table-ownership convention) and the `VideoMetadataRepository` trait (`save(&VideoMetadata, &Path) -> Result<()>`, `find(&VideoRecordId) -> Result<Option<VideoMetadata>>`) in `src/infrastructure/repositories/sqlite_video_metadata_repository.rs`
- [x] 4.2 Implement `SqliteVideoMetadataRepository::save` to write `movie.nfo` into the given video directory first, then persist the DB row only once that write succeeds, and verify a test that a failed file write (e.g. non-existent directory) leaves no DB row
- [x] 4.3 Implement `SqliteVideoMetadataRepository::find` to read only the DB row and verify a test that it returns `None` when no row exists even if a `movie.nfo` file is present on disk
- [x] 4.4 Add a `FakeVideoMetadataRepository` test double, following this repo's existing `Fake*Repository` pattern

## 5. Download-time generation

- [x] 5.1 In `VideoDownloader::download` (`src/domain/services/video_downloader.rs`), after a successful download and thumbnail lookup, fetch `YoutubeMetadata`, resolve `sorttitle`, build `VideoMetadata`, and call `VideoMetadataRepository::save`; on any failure in this sequence, skip the save entirely (no file, no DB row) and continue marking the video `Downloaded` as today
- [x] 5.2 Verify with tests in `video_downloader.rs`/`download_video_task.rs`: successful metadata generation results in a `VideoMetadataRepository` row; a failed `YoutubeMetadataRepository::find` still marks the video `Downloaded` and leaves no row
- [x] 5.3 Remove the blank `meta.nfo` write from `ytdlp::download_video` (`src/infrastructure/shared/ytdlp.rs`) and update its existing tests (`it_should_write_an_empty_meta_nfo_file_in_the_video_folder` and the two tests asserting `meta.nfo` existence) to drop that assertion, verifying `cargo test` passes

## 6. Reconcile-time repair

- [x] 6.1 Add the metadata-repair loop to `VideoReconciler::reconcile_filesystem` (`src/domain/services/video_reconciler.rs`): for each `Downloaded` video with no `VideoMetadataRepository` row, run the same generation steps as 5.1, with no status change and no re-download; add `YoutubeMetadataRepository` and `VideoMetadataRepository` as new constructor dependencies and verify with tests mirroring the existing "unhealthy downloaded video" test structure
- [x] 6.2 Add the equivalent loop to `ChannelVideoReconciler` (`src/domain/services/channel_video_reconciler.rs`) and verify with the channel-flavored equivalent tests
- [x] 6.3 Verify a video already recorded as having populated metadata is left untouched by a reconcile pass (no extra API call, no rewrite) in both reconcilers' tests

## 7. Wiring

- [x] 7.1 Construct `YoutubeApiMetadataRepository` and `SqliteVideoMetadataRepository` in `src/serve.rs`, reusing the existing `youtube_api_key()`, and inject them into `VideoDownloader`, `VideoReconciler`, and `ChannelVideoReconciler`
- [x] 7.2 Verify `cargo build --release`, `cargo test --locked`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass

## 8. End-to-end verification

- [x] 8.1 Run the app locally via `scripts/run-local.sh` against a real playlist/channel with `YOUTUBE_API_KEY` set, download a video, and manually confirm `movie.nfo` contains well-formed XML with the expected fields and a stable `uniqueid`
- [x] 8.2 Manually verify the self-healing path: delete a video's DB metadata row (or simulate a fetch failure) for an already-downloaded video, trigger a reconcile pass (on-demand endpoint or wait for the interval), and confirm `movie.nfo` gets generated without the video re-downloading
