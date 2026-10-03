## 1. Walking skeleton

- [x] 1.1 Add the following stubs from design.md's Types & Signatures, with no behaviour change. Verify that `cargo build` and `cargo test --locked` pass.
  - `Video::is_download_settled` (returns `false`)
  - `ScheduledTask::download_video_id` (returns `None`)
  - private `video_ids_with_download_in_flight` in both reconcilers (returns an empty `HashSet`)

## 2. Behaviour (TDD)

- [x] 2.1 `download_video_task::it_should_skip_the_download_of_an_already_downloaded_video`: implement `is_download_settled` and add the guard plus its `debug!` in `VideoDownloader::download`. If an existing download test starts from a settled video, fix its arrange. Verify with `cargo test download_video_task`.
- [x] 2.2 `download_video_task::it_should_skip_the_download_of_an_excluded_video`: verify with `cargo test it_should_skip_the_download_of_an_excluded_video`.
- [x] 2.3 `download_video_task::it_should_skip_the_download_of_an_errored_video`: verify with `cargo test it_should_skip_the_download_of_an_errored_video`.
- [x] 2.4 `reconcile_playlist_task::it_should_reschedule_the_download_of_an_errored_retrying_video_with_no_download_task`: implement `download_video_id` and `video_ids_with_download_in_flight` (read before the stored videos), then add the stranded loop after the Errored recovery with the `warn!`. Verify with `cargo test reconcile_playlist_task`.
- [x] 2.5 `reconcile_playlist_task::it_should_reschedule_the_download_of_a_pending_video_with_no_download_task`: verify with `cargo test it_should_reschedule_the_download_of_a_pending_video_with_no_download_task`.
- [x] 2.6 `reconcile_playlist_task::it_should_reschedule_the_download_of_an_in_progress_video_with_no_download_task`: verify with `cargo test it_should_reschedule_the_download_of_an_in_progress_video_with_no_download_task`.
- [x] 2.7 `reconcile_playlist_task::it_should_not_reschedule_a_non_terminal_video_whose_download_is_queued` (asserts no "stranded" log too, since dedupe alone already prevents the duplicate task; log capture helper moved to `application/tasks/log_capture.rs`): verify with `cargo test it_should_not_reschedule_a_non_terminal_video_whose_download_is_queued`.
- [x] 2.8 `reconcile_playlist_task::it_should_not_reschedule_a_downloaded_video`: verify with `cargo test it_should_not_reschedule_a_downloaded_video`.
- [x] 2.9 `reconcile_channel_task::it_should_reschedule_the_download_of_an_errored_retrying_video_with_no_download_task`: add the same stranded loop to `ChannelVideoReconciler`. Verify with `cargo test reconcile_channel_task`.
- [x] 2.10 `reconcile_channel_task::it_should_not_reschedule_a_non_terminal_video_whose_download_is_queued`: verify with `cargo test reconcile_channel_task`.

## 3. Infrastructure adapters (TDD)

_None: no adapter changes._

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 After deploying, check the logs:
  - `stranded video found during reconcile` appears for the 6 prod videos.
  - `1rYxtU1PayE` and `8dDFhl_UNTQ` log `excluding permanently-unavailable video`.
  - No further hourly `no thumbnail available` WARN lines appear for those two.
