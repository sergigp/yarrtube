## 1. Walking skeleton

- [x] 1.1 Create/change every file, type and signature in design.md (## Files, ## Types & Signatures): add migration 0003 with only its column adds and the `premiered`/`year` drop (no backfill `UPDATE`s) and register it; add `Video.synced_at` (always `None`); replace `premiered`/`year` with `published_at` on `VideoMetadata` plus `premiered()`/`year()` accessors used by `nfo.rs`; add `created_at`/`updated_at` to `VideoMetadata` and `now` to `VideoMetadata::new`/`build_video_metadata`, with callers in `VideoDownloader` and both reconcilers passing `clock.now()`, but `build_video_metadata` setting both timestamps to `DateTime::UNIX_EPOCH`; read/write `published_at`/`created_at`/`updated_at` in `SqliteVideoMetadataRepository` (upsert still overwriting `created_at`); read/write `synced_at` in `SqliteVideoRepository`; add `VideoResponse.synced_at` hardcoded to `None`. Update existing tests to the new constructors. Verify: `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `download_video_task::it_should_record_the_sync_time`: drives `mark_downloaded` setting `synced_at = Some(now)`. Verify: test red, then green with the full suite passing.
- [x] 2.2 `reconcile_playlist_task::it_should_clear_the_sync_time_of_videos_redownloaded`: drives `reset_for_redownload` clearing `synced_at`. Verify: test red, then green with the full suite passing.
- [x] 2.3 `download_video_task::it_should_record_when_metadata_was_generated`: drives `build_video_metadata` setting `created_at`/`updated_at` from `now`. Verify: test red, then green with the full suite passing.
- [x] 2.4 `download_video_task::it_should_keep_the_metadata_creation_time_when_regenerated`: drives the metadata upsert keeping the stored `created_at` and updating `updated_at`. Verify: test red, then green with the full suite passing.
- [x] 2.5 `http::videos::it_should_include_the_sync_time_when_listing_videos`: drives `VideoResponse::from` mapping `synced_at`. Verify: test red, then green with the full suite passing.

## 3. Infrastructure adapters (TDD)

### sqlite_migrations

- [x] 3.1 `it_should_backfill_the_sync_time_of_downloaded_videos_when_migrating`: adds the `synced_at` backfill for `DOWNLOADED` videos to migration 0003. Verify: test red, then green with the full suite passing.
- [x] 3.2 `it_should_backfill_the_publish_time_from_premiered_when_migrating`: adds the `published_at` backfill from `premiered` (before the column drop). Verify: test red, then green with the full suite passing.
- [x] 3.3 `it_should_backfill_the_metadata_update_time_from_its_creation_time_when_migrating`: adds the `updated_at = created_at` backfill. Verify: test red, then green with the full suite passing.

## 4. Verification

- [x] 4.1 Run `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`. Verify: all pass.
- [ ] 4.2 Run `scripts/run-local.sh` against a copy of an existing `yarrtube.sqlite3`: confirm the migration applies, downloaded videos report `synced_at` in `GET /playlists/{id}/videos`, `video_metadata` rows have `published_at`/`updated_at` and no `premiered`/`year`, and a regenerated `movie.nfo` still has the same `premiered`/`year`. Verify: observed manually.
