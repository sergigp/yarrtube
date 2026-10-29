# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end-to-end: `VideoLock` (+ guard) in `infrastructure/shared/video_lock.rs` and `InfrastructureContainer`; `TaskRepository::find_non_completed_for_video` stub returning `Ok(None)`; `VideoReconciler` with its new dependencies, `VideoOwner`, and the private helpers as pass-through stubs (`reset_if_broken`/`recover_if_due` return the video unchanged, `repair_metadata`/`warn_if_stranded` do nothing); `VideoReconcilerApi` reduced to `reconcile` — `InternalVideoReconciler` keeps its current behaviour for now by calling private equivalents, and its new `reconcile(&DesiredState)` signature, slimmed `DesiredState` and removed `MembershipDelta` compile; `serve.rs` and all test constructors updated. Done when `cargo build` succeeds and all existing tests pass. No new behaviour, no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_reset_a_video_whose_file_is_missing` (sync_video_task) — sync resets a Downloaded video with no file and schedules thumbnail fetch + download.
- [x] 2.2 `it_should_reset_a_video_whose_file_is_not_mp4` (sync_video_task) — same for a `.mkv`.
- [x] 2.3 `it_should_recover_an_errored_video_after_its_cooldown` (sync_video_task) — Errored ≥24h is reset and gets thumbnail fetch + download.
- [x] 2.4 `it_should_schedule_nothing_for_a_settled_errored_video` (sync_video_task, existing) — Errored <24h stays Errored, nothing scheduled.
- [x] 2.5 `it_should_never_recover_an_excluded_video` (sync_video_task) — Excluded stays Excluded, nothing scheduled.
- [x] 2.6 `it_should_generate_missing_metadata_of_a_playlist_video` (sync_video_task) — metadata saved with sorttitle from playlist position.
- [x] 2.7 `it_should_generate_missing_metadata_of_a_channel_video` (sync_video_task) — sorttitle from publish date.
- [x] 2.8 `it_should_keep_existing_metadata` (sync_video_task) — no regeneration when a metadata row exists.
- [x] 2.9 `it_should_relocate_a_legacy_video_when_its_metadata_is_recovered` (sync_video_task) — file set moved into the season folder, filenames recorded.
- [x] 2.10 `it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable` (sync_video_task) — video unchanged.
- [x] 2.11 `it_should_warn_about_a_stranded_video` (sync_video_task) — warning logged, download scheduled.
- [x] 2.12 `it_should_keep_a_stranded_errored_retrying_video_errored_retrying` (sync_video_task) — warning logged, download scheduled, status unchanged.
- [x] 2.13 `it_should_not_warn_about_a_video_whose_download_is_running` (sync_video_task) — no warning, no second download.
- [x] 2.14 `it_should_redownload_videos_with_missing_file` (reconcile_channel_task, update) — pass delegates per-video work to `VideoReconciler::reconcile`; reset video gets thumbnail fetch + download; remove `it_should_schedule_only_the_download_of_a_video_reset_for_redownload` and the per-video recovery code/tests from `internal_video_reconciler.rs` now covered by 2.1–2.13.
- [x] 2.15 `it_should_sync_a_video_added_by_the_same_pass` (reconcile_channel_task) — replaces `it_should_schedule_no_step_for_a_video_added_by_the_same_pass`; steps scheduled once; `handled` set removed.
- [x] 2.16 `it_should_repair_the_categories_of_stored_videos` (reconcile_channel_task, update) — one event per repaired video; `assign_source` removed from the channel/playlist reconcilers and `VideoCategoryAssignerApi`.
- [x] 2.17 `it_should_delete_the_stale_file_of_a_video_reset_by_the_same_pass` (reconcile_channel_task) — orphan sweep reads its snapshot after the syncs.
- [x] 2.18 `it_should_redownload_videos_with_missing_file` (reconcile_playlist_task, update) — as 2.14 for playlists.
- [x] 2.19 `it_should_sync_a_video_added_by_the_same_pass` (reconcile_playlist_task) — replaces `it_should_not_schedule_a_thumbnail_fetch_for_a_video_added_in_the_same_pass`.
- [x] 2.20 `it_should_repair_the_categories_of_stored_playlist_videos` (reconcile_playlist_task, update) — one event per repaired video.
- [x] 2.21 Update step 6 of `doc/ARCHITECTURE.md`: the pass runs `VideoReconciler` for every stored video (categories, broken-file reset, errored recovery, metadata repair, step scheduling), then sweeps orphans. Verify by reading it against design.md's Call Stack.

## 3. Infrastructure adapters (TDD)

### VideoLock

- [x] 3.1 `it_should_block_a_second_lock_of_the_same_video_until_released` — second `lock` of the same id returns only after the first guard drops.
- [x] 3.2 `it_should_not_block_locks_of_different_videos` — two ids lock concurrently.

### SqliteTaskRepository

- [x] 3.3 `it_should_find_a_running_task_of_a_video` — `find_non_completed_for_video` returns the pending or running task matching type and video id.
- [x] 3.4 `it_should_not_find_a_completed_task_of_a_video` — returns `None` for completed tasks and other videos.

## 4. Verification

- [x] 4.1 `cargo test --locked` passes.
- [x] 4.2 `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass.
- [x] 4.3 End-to-end suite passes via `scripts/run-tests.sh`.
- [ ] 4.4 Manual (human): on a real library, delete a downloaded episode file, trigger a reconcile of its source, and confirm it is redownloaded with its thumbnail and NFO and Plex shows the episode again.

## Workflow follow-up

- Archive the change after review.
