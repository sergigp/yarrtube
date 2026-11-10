## 1. Walking skeleton

- [x] 1.1 Add `migrations/0026_video_version.sql`, `Video.version` (read and written by the row mapping, 0 in `Video::create`), `Video::with_title` and `Video::next_version`, `StaleVideo` in `domain/video/errors.rs`, `VideoRepository::insert` (delegating to the current insert-or-replace for now), and `domain/services/video_updater.rs` (`VideoUpdater`, `VideoUpdaterApi::update` returning `Ok(None)`, `MAX_UPDATE_ATTEMPTS`), exported from `services/mod.rs` and constructed in `infrastructure_container.rs`. Old paths (`save`, `update_*`, `VideoLock`) stay untouched. Done when `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

Callers move to `VideoUpdater` (2.6–2.13) before `update` becomes version-checked (2.14–2.17): a versioned `update` would otherwise reject the stale copies the unmigrated services still write. Until 2.14, `VideoUpdater` is a plain find → transition → `update`.

- [x] 2.1 `reconcile_channel_task::it_should_schedule_a_sync_of_every_stored_video`: drives `InternalVideoReconciler` to schedule `Task::SyncVideo` per video (new `task_repository` and `clock` dependencies, replacing `video_reconciler`). The REFACTOR step rewrites the channel reconcile tests that asserted inline sync effects so they assert the scheduled syncs.
- [x] 2.2 `reconcile_playlist_task::it_should_schedule_a_sync_of_every_stored_video`: the same for playlists. The REFACTOR step adapts the playlist reconcile tests, the `reconcile_on_*_created` subscriber tests and the HTTP force-reconcile tests.
- [x] 2.3 `reconcile_channel_task::it_should_leave_a_broken_video_to_its_sync`: the pass no longer resets inline.
- [x] 2.4 `reconcile_playlist_task::it_should_schedule_one_sync_of_a_video_added_by_the_pass`: covers the dedupe with the `VideoAddedToPlaylist` sync.
- [x] 2.5 `sync_video_task::it_should_reset_a_broken_video`: a regression test. The REFACTOR step removes `VideoLock` from `VideoReconciler`, the container and every test, and deletes `infrastructure/shared/video_lock.rs`.
- [x] 2.6 `http::videos::it_should_record_progress_on_every_copy_keeping_their_download_fields`: moves `VideoWatchStateUpdater` (every method) to `VideoUpdater`.
- [x] 2.7 `download_video_task::it_should_keep_watch_state_recorded_during_download`: moves `VideoDownloader` to `VideoUpdater`. The REFACTOR step removes `VideoDownloader::save` and `Video::with_watch_state_of`.
- [x] 2.8 `download_video_task::it_should_skip_if_video_deleted_during_download`: a regression test.
- [x] 2.9 `upgrade_quality_task::it_should_keep_watch_progress_recorded_while_upgrading`: moves `QualityUpgrader` to `VideoUpdater`. The REFACTOR step removes `update_quality`.
- [x] 2.10 `fetch_subtitles_task::it_should_keep_watch_progress_recorded_while_fetching`: moves `SubtitleFetcher`. The REFACTOR step removes `update_subtitles`.
- [x] 2.11 `fetch_chapters_task::it_should_keep_watch_progress_recorded_while_fetching`: moves `ChapterFetcher`. The REFACTOR step removes `update_chapters`.
- [x] 2.12 `summarize_video_task::it_should_keep_watch_progress_recorded_while_summarizing`: moves `VideoSummarizer`. The REFACTOR step removes `update_summary`.
- [x] 2.13 `reconcile_channel_task::it_should_refresh_existing_videos` and `reconcile_playlist_task::it_should_refresh_title_of_existing_videos` (existing, regression; the read → title-write window has no fake to hook a concurrent download into, so that interleaving is left to 2.14): moves the title refresh in both membership reconcilers to `VideoUpdater` + `with_title`. The REFACTOR step removes `update_title`, and moves `ThumbnailFetcher` (`with_thumbnail`), `VideoRelocator` and `VideoReconciler::reset_for_redownload` to `VideoUpdater` (covered by the existing fetch_thumbnail, sync_video and migrate-layout tests), then removes `update_thumbnail`.
- [x] 2.14 `video_updater::it_should_apply_the_transition_on_top_of_a_concurrent_write`: drives the find → transition → `update` loop and the `StaleVideo` retry, plus the minimal versioned `UPDATE ... WHERE version = ?` needed for it.
- [x] 2.15 `video_updater::it_should_skip_if_the_video_is_gone`.
- [x] 2.16 `video_updater::it_should_skip_if_the_transition_declines`.
- [x] 2.17 `video_updater::it_should_fail_if_the_video_keeps_changing`.
- [x] 2.18 `http::channels::it_should_move_a_channel_whose_video_sync_is_pending`: only a running `sync_video` blocks a move (audit finding: the syncs each pass queues refused moves until they drained).

## 3. Infrastructure adapters (TDD)

`SqliteVideoRepository`:

- [x] 3.1 `it_should_insert_a_new_video_at_version_zero`.
- [x] 3.2 `it_should_fail_to_insert_an_existing_video`: `insert` becomes a plain `INSERT`. The REFACTOR step removes `save` and renames the test seeds from `save(` to `insert(`. A seed that overwrites an existing row uses `update` on the found video.
- [x] 3.3 `it_should_bump_the_version_on_update`.
- [x] 3.4 `it_should_fail_to_update_a_stale_video`: `StaleVideo` can be downcast from the error, and the row is unchanged.
- [x] 3.5 `it_should_not_recreate_a_deleted_video_on_update`.
- [x] 3.6 `it_should_bump_the_version_when_assigning_an_episode_number`.
- [x] 3.7 `it_should_bump_the_version_when_resetting_unavailable_summaries`. The REFACTOR step deletes the obsolete `it_should_update_only_*` tests.

## 4. Verification

- [x] 4.1 Run `scripts/run-tests.sh --only web --only rust` (fmt, clippy, tests), and verify that `grep -rn "VideoLock\|update_title\|update_subtitles\|update_summary\|update_thumbnail\|update_chapters\|update_quality\|with_watch_state_of" src` is empty.
- [ ] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, add a short playlist through the HTTP API, wait for a download, then POST progress for it repeatedly while its subtitle and chapter fetches run. Check that `GET /api/tasks` shows `sync_video` tasks after a `POST /api/playlists/{id}/reconcile`, and that the video ends up with both its position and its fetched subtitles in the API response. Then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
