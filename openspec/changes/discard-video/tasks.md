# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every type and signature from design.md's Types & Signatures, wired end-to-end with trivial bodies (no `todo!()`):
  - Domain: `VideoStatus::Discarded` ("DISCARDED"), `Download.discarded` + `is_discarded` / `is_discardable` / `discard` (return `false` / `self`), `Video::discard` (returns `None`) / `is_skipped` (returns `false`), `DiscardVideoError`, `VideoDiscarded` and `VideoFileDeleted` events, `VideoDiscarder` + `VideoDiscarderApi` (returns `Ok(())`) exported from `domain::services`, `VideoFileDeleter::new` taking an `EventPublisher` (not publishing yet), `video_window::within_limit` (returns `uploads` unchanged), `ChannelVideoReconciler::list_current_videos` taking `skipped` (passed as 0).
  - Application: `discard_video` handler (204) routed at `POST /videos/{id}/discard`, `AppState.video_discarder` built in `serve.rs` and the HTTP test support; subscribers `DeleteVideoFileOnVideoDiscarded` and `ScanPlexFolderOnVideoFileDeleted` (return `Ok(())`) registered in `serve.rs` (the Plex one only with Plex configured).
  - Infrastructure: `SqliteVideoRepository` parsing `DISCARDED`.
  - Web: `DISCARDED` in `api/types.ts`, `discardVideo` in `api/client.ts`, `useDiscardVideo` in `api/queries.ts`, `canDiscard` in `lib/videoSteps.ts` (returns `false`), `DiscardVideoButton` placed next to `RetryVideoButton` in `VideoDetail`.
  - Done when `cargo build` succeeds and `scripts/run-tests.sh --lint --web --rust` passes with no new tests.

## 2. Behaviour (TDD)

- [ ] 2.1 `it_should_discard_a_retrying_video` — download Discarded, attempts and failure cleared, its pending download task removed.
- [ ] 2.2 `it_should_cancel_every_pending_task_of_a_discarded_video` — every pending task of the video deleted.
- [ ] 2.3 `it_should_discard_a_failed_video` — a Failed download can be discarded.
- [ ] 2.4 `it_should_discard_a_downloaded_video_and_clear_its_files` — recorded file, thumbnail and subtitle filenames cleared.
- [ ] 2.5 `it_should_publish_video_discarded_for_a_downloaded_video` — `VideoDiscarded` with the owner's output dir and the filenames read before the discard.
- [ ] 2.6 `it_should_publish_nothing_when_discarding_a_video_never_downloaded`.
- [ ] 2.7 `it_should_leave_a_discarded_video_unchanged_when_discarded_again` — 204, nothing written or published.
- [ ] 2.8 `it_should_refuse_to_discard_a_video_being_downloaded` — 409, video and tasks unchanged.
- [ ] 2.9 `it_should_refuse_to_discard_an_unavailable_video` — 409.
- [ ] 2.10 `it_should_fail_to_discard_an_unknown_video` — 404.
- [ ] 2.11 `it_should_omit_discarded_videos_from_a_channel_list` — `VideoSearcher` filters `is_skipped`.
- [ ] 2.12 `it_should_omit_discarded_videos_from_a_playlist_list`.
- [ ] 2.13 `it_should_keep_a_discarded_download_on_retry` — `Download::retry` keeps it discarded; no download scheduled.
- [ ] 2.14 `delete_video_file_on_video_discarded::it_should_schedule_the_deletion_of_the_discarded_video_files` — `DeleteVideoFile` pending now.
- [ ] 2.15 `scan_plex_folder_on_video_file_deleted::it_should_scan_the_show_folder_after_a_deletion`.
- [ ] 2.16 `delete_video_file_task::it_should_publish_video_file_deleted_after_deleting_a_video_file`.
- [ ] 2.17 `delete_video_file_task::it_should_publish_nothing_when_no_file_was_recorded`.
- [ ] 2.18 `sync_video_task::it_should_schedule_nothing_for_a_discarded_video` — `VideoSyncPlan` empty, thumbnail not fetchable.
- [ ] 2.19 `download_video_task::it_should_settle_without_downloading_a_discarded_video` — `is_settled` includes Discarded.
- [ ] 2.20 `upgrade_quality_task::it_should_record_nothing_on_a_video_discarded_during_the_upgrade`.
- [ ] 2.21 `reconcile_channel_task::it_should_keep_a_discarded_video_discarded_on_resync`.
- [ ] 2.22 `reconcile_channel_task::it_should_request_one_more_upload_per_skipped_video`.
- [ ] 2.23 `reconcile_channel_task::it_should_not_count_skipped_videos_toward_the_limit` — `within_limit`.
- [ ] 2.24 `reconcile_channel_task::it_should_evict_uploads_past_the_limit_th_unskipped_one`.
- [ ] 2.25 Web "discards a retrying video after confirming" — `DiscardVideoButton` + `ConfirmDialog` + `canDiscard`; `POST /api/videos/{id}/discard`, video gone from the refetched list.
- [ ] 2.26 Web "warns that a downloaded video's files will be deleted".
- [ ] 2.27 Web "sends nothing when the confirmation is cancelled".
- [ ] 2.28 Web "offers no Don't download for a downloading video".
- [ ] 2.29 Web "tells the user when discarding fails".

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `SqliteVideoRepository::it_should_round_trip_a_discarded_video` — `DISCARDED` decodes to discarded, Pending, no attempts, no file.

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --lint --web --rust`; all suites pass.
- [ ] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a channel via the HTTP API and wait for one video to download. With the Playwright MCP browser: discard a Pending/Retrying video (confirm) and a Downloaded one; assert the confirmation texts, that both leave the channel list, that `GET /api/tasks` shows no pending task for them and a completed `delete_video_file` for the downloaded one, and that its files are gone from the fresh videos dir; screenshot the dialog. Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): on the NAS after deploy, discard the Control de Misión livestream (video `461e8684-45d5-454b-acc1-3261fdb8159a`, YouTube `QaK1s0RObH0`) once it isn't downloading; confirm no further download attempt over the next day, the channel gains a video to fill its limit on the next reconcile, and a discarded downloaded episode disappears from Plex.
