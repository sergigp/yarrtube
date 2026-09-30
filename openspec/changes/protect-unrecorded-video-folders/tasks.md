The implementation and its tests already exist on `bug-download-videps` (commits `148ebdf`..`9c1a8d9`). Each task confirms that the piece is in place and that its test passes; tick it once confirmed.

## 1. Walking skeleton

- [x] 1.1 Confirm every file, type and signature in design.md's `## Files` and `## Types & Signatures` exists as specified (`Video::unrecorded_folder_candidates`, `MembershipChanges` in both reconcilers, `SchedulerState.running_ids`, `Slot.task_id`, `recover_orphaned_tasks`, `recover_running_tasks`, `serve::task_handlers`, `serve::event_subscribers`, `TestDatabase::path`) and that `cargo build` succeeds.

## 2. Behaviour (TDD)

- [ ] 2.1 `reconcile_playlist_task` › `it_should_keep_the_folder_of_a_thumbnail_fetch_in_progress`: the sweep keeps a Pending video's unrecorded folder. Verify with `cargo test it_should_keep_the_folder_of_a_thumbnail_fetch_in_progress`.
- [ ] 2.2 `reconcile_playlist_task` › `it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed`: the sweep keeps the folder named after a renamed video's previous title, and the new title is recorded. Verify with `cargo test it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed`.
- [ ] 2.3 `reconcile_channel_task` › `it_should_keep_the_folder_of_a_thumbnail_fetch_in_progress`: channel equivalent of 2.1. Verify with the same filter.
- [ ] 2.4 `reconcile_channel_task` › `it_should_keep_the_folder_of_a_download_in_progress_if_video_renamed`: channel equivalent of 2.2. Verify with the same filter.
- [ ] 2.5 `task_executor` › `it_should_retry_a_task_left_running_without_waiting_for_a_restart`: a `running` row with no attempt in progress is recovered as a failed attempt on the next pass. Verify with `cargo test it_should_retry_a_task_left_running_without_waiting_for_a_restart`.
- [ ] 2.6 `task_executor` › `it_should_start_the_next_task_as_soon_as_one_finishes`: the run loop runs a pass on start and after every completion. Verify with `cargo test it_should_start_the_next_task_as_soon_as_one_finishes`.
- [ ] 2.7 `serve` › `it_should_register_a_handler_for_every_task_type`: the production handler registry covers all 8 task types. Verify with `cargo test it_should_register_a_handler_for_every_task_type`.
- [ ] 2.8 `serve` › `it_should_register_the_subscribers_of_every_event_type`: the production subscriber registry covers all 8 event types with the right subscriber counts. Verify with `cargo test it_should_register_the_subscribers_of_every_event_type`.

## 3. Infrastructure adapters (TDD)

No adapter changes in this change; nothing to do.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 `cargo mutants` on `TaskExecutor::run`, `tasks::registry` and `subscribers::registry` reports every mutant caught.
- [ ] 4.3 `scripts/run-smoke-tests.sh` passes.
