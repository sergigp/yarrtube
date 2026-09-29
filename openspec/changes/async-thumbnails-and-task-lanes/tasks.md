## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature listed in design.md's `## Files` and `## Types & Signatures`, wire them end to end, and extend the fakes. Bodies are trivial: the old sequential behaviour, or `Ok(())`/`None`. Done when `cargo build` succeeds and all existing tests pass, with no new behaviour and no new tests.
  - **Domain types:** `TaskLane`, `Task::FetchThumbnail` with its payload encoding/decoding, `Task::lane_for` and `Task::exclusivity_key`, `video_folder_candidates` and `collision_suffixed_folder`.
  - **New handlers and subscribers:** `FetchThumbnailTask`, `FetchThumbnailOnVideoAddedToPlaylist`, `FetchThumbnailOnVideoAddedToChannel`, registered in `tasks::registry` and `subscribers::registry` (each before the download subscriber).
  - **Port changes:** `ThumbnailFetcher::new` takes a `TaskRepository` and gains `schedule_missing`. `TaskRepository::claim`, `VideoRepository::update_title` and `update_thumbnail` are added.
  - **Executor:** `TaskExecutor::new(…, download_concurrency)` and `schedule_pass`, still running tasks one at a time.
  - **Fakes:** `FakeVideoDownloaderRepository` gets its `on_download` hook.
  - **Composition:** `serve.rs` gets `download_concurrency()` and passes the new dependencies.

## 2. Behaviour (TDD)

Subscribers:
- [x] 2.1 `fetch_thumbnail_on_video_added_to_playlist` › `it_should_schedule_a_thumbnail_fetch`: a `VideoAddedToPlaylist` schedules a `fetch_thumbnail` task for the video in the playlist's output dir.
- [x] 2.2 `fetch_thumbnail_on_video_added_to_playlist` › `it_should_skip_if_playlist_no_longer_exists`: no task, `Ok(())`.
- [x] 2.3 `fetch_thumbnail_on_video_added_to_channel` › `it_should_schedule_a_thumbnail_fetch`: channel equivalent of 2.1.
- [x] 2.4 `fetch_thumbnail_on_video_added_to_channel` › `it_should_skip_if_channel_no_longer_exists`: channel equivalent of 2.2.

`FetchThumbnailTask`:
- [x] 2.5 `it_should_fetch_and_record_the_thumbnail`: the fetched thumbnail is recorded on the video (whole-row assert).
- [x] 2.6 `it_should_skip_if_video_already_has_a_thumbnail`: no fetcher call, row unchanged.
- [x] 2.7 `it_should_skip_if_video_no_longer_exists`: `Ok(())`, no fetcher call.
- [x] 2.8 `it_should_succeed_without_recording_if_thumbnail_fetch_fails`: best-effort, `Ok(())`, row unchanged.

`ReconcilePlaylistTask`:
- [x] 2.9 `it_should_not_fetch_thumbnails_when_adding_new_videos`: new videos are stored `Pending` without thumbnails, events are published, and there are zero fake thumbnail calls. Update existing tests that expected inline thumbnails.
- [x] 2.10 `it_should_schedule_a_thumbnail_fetch_for_a_video_missing_one`: recovery schedules a `fetch_thumbnail` task instead of fetching.
- [x] 2.11 `it_should_not_schedule_a_second_thumbnail_fetch_if_one_is_queued`: still exactly one `fetch_thumbnail` task.
- [ ] 2.12 `it_should_not_schedule_a_thumbnail_fetch_for_a_video_being_downloaded`: an `InProgress` video gets no task.
- [ ] 2.13 `it_should_keep_the_folder_of_a_download_in_progress`: the bare-title folder of an `InProgress` video without a thumbnail is not deleted.
- [ ] 2.14 `it_should_keep_the_suffixed_folder_of_a_download_in_progress`: the `"{title} [{id}]"` folder is not deleted.
- [ ] 2.15 `it_should_still_delete_a_folder_no_video_accounts_for`: the orphan sweep still removes unaccounted folders.

`ReconcileChannelTask`:
- [ ] 2.16 `it_should_not_fetch_thumbnails_when_adding_new_videos`: channel equivalent of 2.9.
- [ ] 2.17 `it_should_schedule_a_thumbnail_fetch_for_a_video_missing_one`: channel equivalent of 2.10.
- [ ] 2.18 `it_should_keep_the_folder_of_a_download_in_progress`: channel equivalent of 2.13.

Download and task listing:
- [ ] 2.19 `download_video_task` › `it_should_remove_the_folder_if_video_deleted_during_download`: the video is deleted mid-download via the fake's `on_download` hook. The download's folder is removed and nothing is recorded.
- [ ] 2.20 `http/tasks` › `it_should_list_a_fetch_thumbnail_task_with_video_and_playlist_names`: `TaskViewSearcher` resolves `fetch_thumbnail` context.

`TaskExecutor`:
- [ ] 2.21 `it_should_run_downloads_in_parallel_up_to_the_lane_concurrency`: the download lane honours its concurrency and refills when a slot frees.
- [ ] 2.22 `it_should_not_block_light_tasks_behind_downloads`: the light lane is independent of a busy download lane.
- [ ] 2.23 `it_should_run_light_tasks_one_at_a_time`: light lane concurrency is 1.
- [ ] 2.24 `it_should_not_block_reconciles_behind_thumbnail_fetches`: a thumbnail backlog doesn't hold the light lane.
- [ ] 2.25 `it_should_run_thumbnail_fetches_one_at_a_time`: thumbnail lane concurrency is 1.
- [ ] 2.26 `it_should_not_run_two_tasks_for_the_same_video_at_once`: a held exclusivity key blocks a task for the same video.
- [ ] 2.27 `it_should_start_a_later_task_when_the_earlier_one_is_held_back`: a `fetch_thumbnail` held back by its video's running download doesn't block the next fetch in the thumbnail lane.
- [ ] 2.28 `it_should_run_update_ytdlp_only_when_nothing_else_runs`: the exclusive drain. Nothing new starts while the update waits, and it runs alone.
- [ ] 2.29 `it_should_not_start_tasks_while_update_ytdlp_runs`: no task starts during the update.
- [ ] 2.30 `it_should_start_tasks_of_a_lane_in_run_at_order`: `run_at, id` ordering within a lane. Adapt the existing dispatch, retry, dead-letter, last-attempt and recovery tests to `schedule_pass` plus awaiting the handles, and keep them green.

Domain and composition:
- [x] 2.31 `task.rs` › `it_should_map_task_types_to_lanes`: `download_video` → Download, `fetch_thumbnail` → Thumbnail, `update_ytdlp` → Exclusive, others → Light.
- [x] 2.32 `task.rs` › `it_should_key_video_tasks_by_video_id`: `download_video`/`fetch_thumbnail` → `Some("video:<id>")`, others → `None`.
- [ ] 2.33 `serve.rs` › `it_should_default_download_concurrency_when_invalid`: unset, `0`, `-1` and `abc` → 2, and `4` → 4.

## 3. Infrastructure adapters (TDD)

`SqliteTaskRepository`:
- [ ] 3.1 `it_should_claim_a_pending_task`: `claim` flips `pending` to `running` and returns the task.
- [ ] 3.2 `it_should_not_claim_a_task_already_running`: `claim` returns `None`.
- [x] 3.3 `it_should_not_schedule_a_duplicate_download_for_the_same_video`: `INSERT … WHERE NOT EXISTS` on type + `json_extract(payload,'$.video_id')`.
- [x] 3.4 `it_should_not_schedule_a_duplicate_thumbnail_fetch_for_the_same_video`: the same dedupe for `fetch_thumbnail`.
- [x] 3.5 `it_should_schedule_a_download_for_another_video`: different video ids both get rows.
- [x] 3.6 `it_should_schedule_duplicates_of_tasks_without_a_video`: `reconcile_playlist` is not deduped.

`SqliteVideoRepository`:
- [ ] 3.7 `it_should_update_only_the_title`: `update_title` leaves status and download fields intact.
- [ ] 3.8 `it_should_update_only_the_thumbnail`: `update_thumbnail` leaves status and download fields intact.

`ytdlp.rs`:
- [ ] 3.9 `it_should_give_concurrent_same_title_videos_distinct_folders`: atomic `create_dir` in `prepare_video_dir`, falling back to the suffixed name. The existing collision and retry-folder tests stay green.

Composition, frontend and docs (no tests):
- [ ] 3.10 Wire the real lane scheduler into `TaskExecutor::run`: timer tick plus `Notify` on completion. Verify with `scripts/run-local.sh`: the logs show `dispatching task` for two `download_video` tasks before either finishes.
- [ ] 3.11 Add the `fetch_thumbnail` case to `web/src/components/TasksView.jsx` ("Fetching thumbnail of {video} in {container}", with generic placeholders). Verify that `npm run build` and `npm run lint` in `web/` pass.
- [ ] 3.12 Document `YARRTUBE_DOWNLOAD_CONCURRENCY` in `README.md` (default 2, throttling note). Verify the README env table lists it.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 `scripts/run-smoke-test.sh` passes.
- [ ] 4.3 Manual check with `scripts/run-local.sh`: add a playlist with 20+ videos.
  - Within seconds, the tasks tab lists `fetch_thumbnail` and `download_video` tasks.
  - Two downloads run at once.
  - Thumbnails fill in while downloads progress.
  - No stray folders are left in `videos/` after the downloads complete.
