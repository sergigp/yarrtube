# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (`VideoTarget`, `VideoSyncStep`, `VideoSyncPlan`, `VideoReconciler` + `VideoReconcilerApi`, `Task::SyncVideo` + `decode_sync_video_payload` + `dedupes_running`, `TaskLane::VideoSync`, `SyncVideoTask`, `SyncVideoOnVideoEvent`), change `InternalVideoReconciler::new` to take a `VideoReconciler`, remove `schedule_missing` and the `task_repository` dep from `ThumbnailFetcher`/`SubtitleFetcher`, wire `sync_video` in the task registry and `SyncVideoOnVideoEvent` in the subscriber registry for `VideoAddedToPlaylist`/`VideoAddedToChannel`/`VideoDownloaded` (removing the five replaced subscribers and their files), and build `VideoReconciler` in `src/serve.rs`. Bodies trivial (`Ok(())`, empty plan, `false`); `InternalVideoReconciler` keeps calling its existing private scheduling until 2.13. Done when `cargo build` succeeds and all existing tests pass (tests of deleted subscribers go with them).

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_schedule_the_download_and_thumbnail_of_a_new_channel_video` — sync resolves the channel target and schedules download + thumbnail fetch.
- [x] 2.2 `it_should_schedule_the_download_of_a_new_playlist_video_in_its_playlist_folder` — playlist owner resolution.
- [x] 2.3 `it_should_schedule_the_subtitles_of_a_downloaded_video` — subtitle step from stored state only.
- [x] 2.4 `it_should_schedule_nothing_for_a_fully_synced_video` — finished steps are never redone.
- [ ] 2.5 `it_should_not_schedule_a_second_download_if_one_is_queued` — dedupe via the task queue.
- [ ] 2.6 `it_should_not_schedule_a_thumbnail_for_a_video_being_downloaded` — InProgress guard.
- [ ] 2.7 `it_should_schedule_nothing_for_a_settled_errored_video` — settled-status guard.
- [ ] 2.8 `it_should_skip_if_video_is_gone` — missing record is a no-op.
- [ ] 2.9 `it_should_skip_if_video_has_no_owner` — orphan record is a no-op.
- [ ] 2.10 `it_should_schedule_a_video_sync_when_a_video_is_added_to_a_channel` — subscriber schedules `sync_video`.
- [ ] 2.11 `it_should_schedule_a_video_sync_when_a_video_is_added_to_a_playlist` — subscriber registered for playlist additions.
- [ ] 2.12 `it_should_schedule_a_video_sync_when_a_video_is_downloaded` — subscriber registered for downloads (replaces the subtitle subscriber).
- [ ] 2.13 `it_should_schedule_only_the_download_of_a_video_reset_for_redownload` — `InternalVideoReconciler` delegates resets to `VideoReconciler::schedule_download` and every other stored video to `reconcile_video`; remove its own stranded/thumbnail/subtitle scheduling. Existing reconcile suites must stay green.
- [ ] 2.14 `it_should_schedule_no_step_for_a_video_added_by_the_same_pass` — added videos are left to their sync.
- [ ] 2.15 `it_should_list_a_video_sync_with_its_video_title_and_owner_name` — `TaskViewSearcher` resolves `sync_video`.
- [ ] 2.16 `it_should_run_video_syncs_in_their_own_lane` — `Task::lane_for("sync_video")`.
- [ ] 2.17 `it_should_give_a_video_sync_the_video_exclusivity_key` — per-video mutual exclusion.
- [ ] 2.18 `describes a video sync with its video and container` (web, `tasks.test.ts`).
- [ ] 2.19 `puts video syncs in the syncs family` (web, `tasks.test.ts`).

## 3. Infrastructure adapters (TDD)

SqliteTaskRepository:
- [ ] 3.1 `it_should_not_add_a_video_sync_if_one_is_pending`
- [ ] 3.2 `it_should_add_a_video_sync_if_the_only_one_is_running` — uses `Task::dedupes_running`.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `npm run check` in `web/`.
- [ ] 4.2 End-to-end suite via `scripts/run-tests.sh`.
- [ ] 4.3 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, add a short playlist via the HTTP API, and confirm via `GET /api/tasks` and the tasks view (Playwright) that a `sync_video` per new video appears in the Syncs tab described as "Syncing <title> in <playlist>", followed by its download and thumbnail fetch, then a subtitle fetch after the download; confirm the video ends Downloaded with its `.srt`. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): with the Plex scan integration enabled, confirm a newly downloaded episode and its subtitle still appear in Plex after the scan.
