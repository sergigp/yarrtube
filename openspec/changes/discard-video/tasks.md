# Tasks

## 1. Walking skeleton

- [ ] 1.1 Create every type and signature from design.md's Types & Signatures, wired end-to-end with trivial bodies (no `todo!()`): `VideoStatus::Discarded` ("DISCARDED"), `Download.discarded` + `is_discarded` / `is_discardable` / `discard` / `undo_discard` (return `self` / `false`), `Video::discard` / `undo_discard` (return `None`), `DiscardVideoError` / `UndoDiscardVideoError`, `VideoDiscarder` + `VideoDiscarderApi` (return `Ok(())`) exported from `domain::services`, `discard_video` / `undo_discard_video` handlers (204) routed at `/videos/{id}/discard` (POST / DELETE), `AppState.video_discarder` built in `serve.rs` and in the HTTP test support, `SqliteVideoRepository` parsing `DISCARDED`; web: `DISCARDED` in `api/types.ts`, `discardVideo` / `undoDiscardVideo` in `api/client.ts`, `useDiscardVideo` / `useUndoDiscardVideo` in `api/queries.ts`, `canDiscard` in `lib/videoSteps.ts` (returns `false`), `STATUS_LABELS.DISCARDED` / `STATUS_MESSAGES.DISCARDED`, `DiscardVideoButton` (renders `null`) placed next to `RetryVideoButton` in `VideoDetail`. Done when `cargo build` succeeds and `scripts/run-tests.sh --lint --web --rust` passes with no new tests.

## 2. Behaviour (TDD)

- [ ] 2.1 `it_should_discard_a_retrying_video` — discard marks the download Discarded with attempts and failure cleared and removes its pending download task.
- [ ] 2.2 `it_should_cancel_every_pending_task_of_a_discarded_video` — every pending task of the video (thumbnail, download) is deleted.
- [ ] 2.3 `it_should_discard_a_failed_video` — a Failed download with 11 attempts can be discarded.
- [ ] 2.4 `it_should_leave_a_discarded_video_unchanged_when_discarded_again` — idempotent 204, nothing written.
- [ ] 2.5 `it_should_refuse_to_discard_a_video_being_downloaded` — In Progress → 409, video and tasks unchanged.
- [ ] 2.6 `it_should_refuse_to_discard_a_downloaded_video` — 409, file kept.
- [ ] 2.7 `it_should_refuse_to_discard_an_unavailable_video` — 409.
- [ ] 2.8 `it_should_fail_to_discard_an_unknown_video` — 404.
- [ ] 2.9 `it_should_undo_a_discard` — download Pending with no attempts, `SyncVideo` scheduled now.
- [ ] 2.10 `it_should_leave_a_video_that_is_not_discarded_unchanged_on_undo` — 204, video unchanged, no task scheduled.
- [ ] 2.11 `it_should_fail_to_undo_the_discard_of_an_unknown_video` — 404.
- [ ] 2.12 `it_should_list_a_discarded_video_with_its_steps` — status and `download` step `DISCARDED` with no attempts, next attempt or failure.
- [ ] 2.13 `it_should_keep_a_discarded_download_on_retry` — retry leaves the download Discarded, other failed steps Pending, no download scheduled.
- [ ] 2.14 `sync_video_task::it_should_schedule_nothing_for_a_discarded_video` — `VideoSyncPlan` is empty for a Discarded video (no thumbnail, no download).
- [ ] 2.15 `download_video_task::it_should_settle_without_downloading_a_discarded_video` — `VideoDownloader::is_settled` includes Discarded; no yt-dlp call, task not re-queued.
- [ ] 2.16 `reconcile_channel_task::it_should_keep_a_discarded_video_discarded_on_resync` — title refreshed, download stays Discarded, no download scheduled.
- [ ] 2.17 Web `VideoDetail.test.tsx` "discards a retrying video from Don't download" — `DiscardVideoButton` + `canDiscard`; refetched video reads "Discarded", no "Don't download", no "Retry now", no failing steps.
- [ ] 2.18 Web `VideoDetail.test.tsx` "offers no Don't download for a downloaded or downloading video".
- [ ] 2.19 Web `VideoDetail.test.tsx` "tells the user when discarding fails" — alert, video as before.
- [ ] 2.20 Web `VideoDetail.test.tsx` "downloads a discarded video anyway" — "Download anyway" sends `DELETE /api/videos/{id}/discard`; video reads "Pending" with "Don't download" shown.
- [ ] 2.21 Web `VideoDetail.test.tsx` "tells the user when undoing a discard fails".
- [ ] 2.22 Web `VideoDetail.test.tsx` "offers no Retry now for a discarded video" — `canRetry` false while `DISCARDED`.
- [ ] 2.23 Web `VideoListPane.test.tsx` "marks a discarded video with its own indicator" — "You chose not to download this video.", no warning icon.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 `SqliteVideoRepository::it_should_round_trip_a_discarded_download` — `DISCARDED` encodes/decodes to discarded, Pending, no attempts.

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --lint --web --rust`; all suites pass.
- [ ] 4.2 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a channel via the HTTP API, pick a video that is Pending or Retrying, open it with the Playwright MCP browser, activate "Don't download" and assert the pane reads "Discarded", `GET` the channel's videos returns it `DISCARDED`, and `GET /api/tasks` shows no pending task for it; activate "Download anyway" and assert it reads "Pending"/"Downloading" and a sync/download task is queued; screenshot both states. Stop the server, delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.3 Manual (human): on the NAS, after deploy, discard the Control de Misión livestream (video `461e8684-45d5-454b-acc1-3261fdb8159a`, YouTube `QaK1s0RObH0`) and confirm no further download attempt shows in the tasks view or logs over the next day; judge the Discarded indicator visually in light and dark mode.
