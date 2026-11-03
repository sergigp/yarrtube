## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature in design.md (## Files, ## Types & Signatures):
  - migration 0021 and its registration
  - `LateDataOutcome`, `LateDataChecks`
  - the new `Video` fields and transitions; remove `subtitles_first_missing_at` / `mark_subtitles_first_missing` and update their callers and tests
  - `Quality::height_cap`
  - `VideoSyncPlan::for_video(video, now)`
  - `VideoSyncStep::UpgradeQuality`, `Task::UpgradeQuality` with its lane and key
  - `ChaptersProbe`, `QualityProbe`, `DownloadedVideo.height`, `probe_quality` on the port, the yt-dlp adapter and the fakes
  - `VideoRepository::update_quality`
  - `QualityUpgrader` and `UpgradeQualityTask`, registered in `serve.rs`
  - `task_view_searcher` and `web/src/lib/tasks.ts` entries

  Bodies return trivial values (the new transitions return `self`, `probe_quality` returns `Ok(QualityProbe { height: None, published_at: None })`, `upgrade` returns `Ok(())`). Done when `cargo build` succeeds and `cargo test --locked` passes with the existing tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_mark_subtitles_unavailable_on_first_check_of_an_old_video` — an old video with no captions is Unavailable after one check (fetch_subtitles_task test passes)
- [x] 2.2 `it_should_keep_subtitles_missing_for_a_young_video_without_captions` — Missing, last check recorded, no failed attempts (test passes)
- [x] 2.3 `it_should_count_a_failed_attempt_if_the_probe_warns` — an inconclusive probe counts a failed attempt (test passes)
- [x] 2.4 `it_should_count_a_failed_attempt_if_the_probe_fails` — a downloader error counts a failed attempt (test passes)
- [x] 2.5 `it_should_mark_subtitles_unavailable_after_three_failed_checks_of_an_old_video` — failures on an old video are bounded (test passes)
- [x] 2.6 `it_should_keep_retrying_failed_checks_of_a_young_video` — failures on a young video never settle it (test passes)
- [x] 2.7 `it_should_settle_chapters_found` — fetched chapters are settled (fetch_chapters_task test passes)
- [x] 2.8 `it_should_keep_empty_chapters_unsettled_for_a_young_video` — a young video with no chapters stays unsettled (test passes)
- [x] 2.9 `it_should_settle_empty_chapters_of_an_old_video` — an old video with no chapters is settled (test passes)
- [x] 2.10 `it_should_count_a_failed_chapters_attempt` — a failed chapters fetch keeps the chapters and counts an attempt (test passes)
- [x] 2.11 `it_should_settle_chapters_as_none_after_three_failed_checks_of_an_old_video` — bounded chapters failures (test passes)
- [x] 2.12 `it_should_skip_settled_chapters` — no downloader call for settled chapters (test passes)
- [x] 2.12a `it_should_recheck_unsettled_empty_chapters` — a young video's empty, unsettled chapters are fetched again and become the newly reported ones (spec scenario "Young video gains chapters later"; fetch_chapters_task test passes)
- [x] 2.13 `it_should_record_the_downloaded_height` — a download records its height (download_video_task test passes)
- [x] 2.14 `it_should_leave_quality_unsettled_for_a_young_video_below_its_cap` — young + below cap stays unsettled (test passes)
- [x] 2.15 `it_should_settle_quality_at_the_tier_cap` — reaching the mid/low cap settles (test passes)
- [x] 2.16 `it_should_settle_quality_of_an_old_video` — an old download settles (test passes)
- [x] 2.17 `it_should_leave_empty_chapters_unsettled_for_a_young_video` — download-time chapters follow the policy (test passes)
- [x] 2.18 `it_should_replace_the_file_with_a_higher_quality_download` — in-place upgrade keeps subtitles, chapters and watch state, and publishes `VideoDownloaded` (upgrade_quality_task test passes)
- [x] 2.19 `it_should_record_the_check_if_nothing_better_is_available` — no download, check recorded (test passes)
- [x] 2.20 `it_should_settle_quality_once_the_video_is_old` — quality settles once the video is old (test passes)
- [x] 2.21 `it_should_count_a_failed_attempt_if_the_probe_fails` (upgrade) — a probe error counts an attempt (test passes)
- [x] 2.22 `it_should_keep_the_file_if_the_redownload_fails` — a failed re-download leaves the file and fields, task `Ok` (test passes)
- [x] 2.22a `it_should_keep_the_file_if_the_redownload_is_no_better` — (audit) a re-download no taller than the recorded height is discarded and counted as a failed attempt (upgrade_quality_task test passes)
- [x] 2.23 `it_should_skip_the_upgrade_if_the_video_was_reset_meanwhile` — a reset during the upgrade places nothing (test passes)
- [x] 2.23a `it_should_skip_the_upgrade_if_the_video_was_redownloaded_meanwhile` — (audit) still Downloaded but renamed meanwhile → nothing placed (upgrade_quality_task test passes)
- [x] 2.24 `it_should_skip_a_video_with_settled_quality` — no probe for settled quality (test passes)
- [x] 2.25 `it_should_not_schedule_subtitles_checked_less_than_a_day_ago` — 24h throttle on subtitles (sync_video_task test passes)
- [x] 2.26 `it_should_schedule_subtitles_checked_more_than_a_day_ago` — due subtitles are scheduled (test passes)
- [x] 2.27 `it_should_schedule_chapters_of_a_young_video_checked_a_day_ago` — due unsettled chapters are scheduled (test passes)
- [x] 2.27a `it_should_not_schedule_chapters_checked_less_than_a_day_ago` — 24h throttle on unsettled chapters (sync_video_task test passes)
- [x] 2.28 `it_should_not_schedule_settled_chapters` — settled empty chapters aren't rechecked (test passes)
- [x] 2.29 `it_should_schedule_a_quality_check_when_due` — the `UpgradeQuality` task is scheduled with the owner's quality and output dir (test passes)
- [x] 2.30 `it_should_not_schedule_a_quality_check_right_after_download` — throttle after download (test passes)
- [x] 2.31 `it_should_reset_every_check_on_redownload` — a broken-file reset clears the checks and height (test passes)
- [x] 2.32 `it_should_list_an_upgrade_quality_task_with_its_video_context` — task listing resolves the title and owner (http tasks test passes)
- [x] 2.33 (audit) `it_should_fetch_the_original_language_subtitles` asserts the Complete check; `Video::with_subtitles` renamed `mark_subtitles_fetched` and records it (fetch_subtitles_task test passes)
- [x] 2.34 `it_should_settle_high_quality_at_1080` — a young `high` video downloaded at 1080 settles; rename `Quality::height_cap` → `target_height` (High 1080, Mid 720, Low 480) (download_video_task test passes)
- [x] 2.35 `it_should_replace_the_file_with_a_higher_quality_download` expects the quality settled once the upgrade reaches 1080 on `high` (upgrade_quality_task test passes)
- [x] 2.36 `it_should_count_a_degraded_download_as_a_failed_quality_check` — an old video's SABR-degraded download records tier and height but counts a failed check instead of settling (download_video_task test passes)
- [x] 2.37 `it_should_count_a_degraded_probe_as_a_failed_check` — a SABR-degraded probe that finds nothing better counts a failed attempt (upgrade_quality_task test passes)
- [x] 2.38 `it_should_count_a_degraded_upgrade_as_a_failed_check` — a SABR-degraded but taller re-download replaces the file and counts a failed attempt (upgrade_quality_task test passes)

## 3. Infrastructure adapters (TDD)

- [x] 3.1 sqlite_migrations: `it_should_settle_checked_chapters_and_downloaded_quality` — backfill of the settled flags (test passes)
- [x] 3.2 sqlite_migrations: `it_should_keep_the_first_missing_time_as_the_last_subtitles_check` — the renamed column keeps its value (test passes)
- [x] 3.3 sqlite_video_repository: `it_should_round_trip_late_data_checks` — every new column maps both ways (test passes)
- [x] 3.4 sqlite_video_repository: `it_should_update_only_quality_fields` — `update_quality` touches only its fields (test passes)
- [x] 3.5 ytdlp: `it_should_report_the_downloaded_height` — the download prints and parses the height (test passes)
- [x] 3.6 ytdlp: `it_should_report_the_chapters_publish_time` — `fetch_chapters` returns `ChaptersProbe` with the publish time (test passes)
- [x] 3.7 ytdlp: `it_should_probe_the_height_for_a_quality` — the tier selector runs with `--skip-download`, and the height and publish time are parsed (test passes)
- [x] 3.8 ytdlp: `it_should_fail_a_quality_probe_on_a_clean_failure` — a non-zero exit returns `Err` (test passes)
- [x] 3.9 web `tasks.test.ts`: `upgrade_quality` is described as a quality check under downloads, with a placeholder when unresolved (`npm run test` passes)
- [x] 3.10 (audit) task: `it_should_run_upgrade_quality_in_the_download_lane_exclusive_per_video`, `it_should_read_the_output_dir_of_an_upgrade_quality` — upgrade_quality lane/key/output dir (task tests pass)
- [x] 3.11 ytdlp: `it_should_report_a_degraded_quality_probe` — the probe keeps warnings and flags the SABR signal on stderr (test passes; `it_should_probe_the_height_for_a_quality` args updated)

## 4. Verification

- [x] 4.1 `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked` are all clean
- [x] 4.2 `scripts/run-tests.sh` (no flags) passes every suite
- [ ] 4.3 Manual (agent):
  - Start `scripts/run-local.sh --port <free port> --fresh-videos`.
  - Add a playlist with an old video that has no captions.
  - Wait for the download and its subtitle fetch.
  - Check through `/api/tasks` and the DB: the video is `UNAVAILABLE` after one fetch, and a second reconcile within 24h queues no `fetch_subtitles` / `fetch_chapters` / `upgrade_quality` for it.
  - Open the tasks view with the Playwright MCP and check that an `upgrade_quality` task (seeded via a young video's sync) is described under downloads.
  - Stop the server, then delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.4 Manual (human):
  - After deploying to the NAS, confirm the ~540 `fetch_subtitles` backlog drains within one pass and doesn't refill.
  - Confirm Plex picks up an in-place quality upgrade of a freshly published video: same episode, higher resolution.
