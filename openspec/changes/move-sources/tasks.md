# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures): `ShowKey`, `MoveShowError`, `ShowMoved` + `DomainEvent::ShowMoved`, `Playlist/Channel::with_path`, `Task::output_dir_of`, `ScheduledTask::output_dir`, `ShowLocks`/`ShowLockGuard`, `VideoFileRepository::{dir_exists, rename_dir}` (+ fake recording renames and a failing constructor), `ShowMover` + `ShowMoverApi`, `ShowLocks` passed to both reconcilers, `move_playlist`/`move_channel` handlers + request DTOs + `move_show_error` mapping + routes, `ScanPlexFolderOnShowMoved` registered when Plex is configured, shared `Arc<ShowLocks>` in the container and `serve.rs`; web `movePlaylist`/`moveChannel`, `useSaveLocation` `initialPath` param, `MoveSourceDialog` stub. Trivial bodies only. Verify: `cargo build` and `cargo test --locked` pass, `npm run check` passes in `web/`.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_move_a_playlist`: renames the show dir, stores the new path, publishes `ShowMoved`, returns the moved playlist.
- [x] 2.2 `it_should_keep_the_pending_reconcile_when_moving`: the pending recurring reconcile is untouched.
- [x] 2.3 `it_should_move_a_playlist_with_errored_and_excluded_videos`: Errored/Excluded count as settled.
- [x] 2.4 `it_should_record_the_new_path_if_the_show_directory_is_missing`: no rename, new path stored.
- [x] 2.5 `it_should_fail_to_move_if_path_missing`: 400.
- [x] 2.6 `it_should_fail_to_move_if_path_invalid`: 400.
- [x] 2.7 `it_should_fail_to_move_a_missing_playlist`: 404.
- [x] 2.8 `it_should_fail_to_move_to_the_same_path`: 409.
- [x] 2.9 `it_should_fail_to_move_to_a_path_used_by_a_channel`: 409.
- [x] 2.10 `it_should_fail_to_move_into_its_own_path`: 409.
- [x] 2.11 `it_should_fail_to_move_inside_another_playlist`: 409.
- [x] 2.12 `it_should_fail_to_move_if_destination_exists_on_disk`: 409, nothing renamed.
- [x] 2.13 `it_should_fail_to_move_if_a_video_is_pending`: 409.
- [x] 2.14 `it_should_fail_to_move_if_a_thumbnail_fetch_is_queued`: 409 driven by a task whose `output_dir` is the show dir.
- [x] 2.15 `it_should_fail_to_move_while_reconciling`: show lock held, 409.
- [x] 2.16 `it_should_fail_to_move_if_rename_fails`: 500, path unchanged, no event.
- [x] 2.17 `it_should_rename_back_if_recording_the_path_fails`: 500, renamed back, no event.
- [x] 2.18 `it_should_move_a_channel`: channel handler wired end to end.
- [x] 2.19 `it_should_fail_to_move_a_missing_channel`: 404.
- [x] 2.20 `it_should_fail_to_move_if_a_channel_video_is_pending`: 409 via channel videos.
- [x] 2.21 `it_should_wait_for_a_move_before_reconciling` (reconcile_playlist_task): reconcile blocks on the lock and then reconciles the new path.
- [x] 2.22 `it_should_wait_for_a_move_before_reconciling` (reconcile_channel_task): same for channels.
- [x] 2.23 `it_should_scan_the_new_and_old_folders` (scan_plex_folder_on_show_moved).
- [x] 2.24 `it_should_fail_if_plex_is_unreachable` (scan_plex_folder_on_show_moved): error returned so the event retries.
- [x] 2.25 `task.rs`: `it_should_read_the_output_dir_of_every_per_video_task` and `it_should_have_no_output_dir_for_reconciles`.
- [x] 2.26 `video.rs`: `it_should_be_settled_only_when_downloaded_errored_or_excluded` (on the reused `is_download_settled`).
- [x] 2.27 Web: `MoveSourceDialog` tests (prefilled location, save disabled when unchanged, posts the move and closes, shows server error and stays open, cancel sends nothing); implement the dialog and `useSaveLocation` `initialPath` (current path not flagged as occupied, parent not remembered). Verify: `npm run check`.
- [x] 2.28 Web: `EditPlaylistDialog`/`EditChannelDialog` tests (Location line shows current path, "Move…" opens the move dialog without saving); implement. Verify: `npm run check`.
- [x] 2.29 Docs: describe moving a playlist/channel, its preconditions and the same-filesystem limit in `doc/INSTALLATION.md` (or the usage section it links), and the post-move scan in `doc/PLEX.md`. Verify: the documented steps match the UI.

## 3. Infrastructure adapters (TDD)

`infrastructure/shared/show_locks.rs`
- [x] 3.1 `it_should_refuse_try_lock_while_held`
- [x] 3.2 `it_should_release_on_drop`
- [x] 3.3 `it_should_block_lock_until_released`
- [x] 3.4 `it_should_not_contend_across_keys`

`infrastructure/repositories/filesystem_video_file_repository.rs`
- [x] 3.5 `it_should_rename_a_directory_creating_missing_parents`
- [x] 3.6 `it_should_fail_to_rename_onto_an_existing_directory`
- [x] 3.7 `it_should_report_whether_a_directory_exists`

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `npm run check` in `web/` all pass.
- [x] 4.2 Smoke tests: add specs in `smoke-tests/tests/` moving a playlist and a channel through the edit dialog's "Move…" (covers both new routes for the route-coverage reporter); `scripts/run-smoke-tests.sh` passes.
- [x] 4.3 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`; create a small playlist via the API and wait until all its videos and thumbnails are downloaded; in the Playwright MCP browser open its edit dialog, check Location, use "Move…" to move it to `archive/<name>`; assert the sidebar/listing shows the new path, the files exist under the new dir and not the old one, a video still plays; then trigger "Sync now" and assert no re-download task appears. Also assert a move attempted while a video is still downloading shows the conflict error in the dialog. Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human): with Plex configured, move a source and confirm Plex shows it at the new location and drops the old show; confirm the move dialog's folder browser looks right on mobile.
