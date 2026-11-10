# Tasks

Behaviour-preserving refactor: no walking skeleton or red-green cycles (the
config's TDD structure assumes new behaviour). Each group is one commit,
green on its own; "green" = `scripts/run-tests.sh --only web --only rust`
passes with no assertion changed in acceptance tests. One PR for all.

## 1. Test fixtures and WatchState

- [x] 1.1 Create `video_fixtures.rs` (`#[cfg(test)]`): move `mark_download_failed`, `mark_summary_failed`, `mark_categorization_failed`, `with_chapters`, `mark_subtitles_unavailable` out of `video.rs`; add `downloaded_video(youtube_id, filename, now)`; verify `cargo test --locked` passes
- [x] 1.2 Add `WatchState` (`watch_state.rs`), replace `watched_at`/`playback_position`/`last_played_at` with `watch`; move `is_watched`/`is_in_progress` reads to `watch`; repository maps the same three columns; verify green
- [x] 1.3 Commit "Group the watch state"

## 2. Subtitles, Chapters, DownloadedQuality

- [ ] 2.1 Add `late_video_data.rs` (constants) and `Subtitles`, replacing `subtitles` + `subtitle_filename`; move `subtitle_language` to `Subtitles::language`; move the `late_data_checks.rs` unit tests onto `Subtitles`; verify green
- [ ] 2.2 Add `Chapters`, replacing `chapters` + `chapters_checks` (keep None vs Some(vec![]); `reset_checks` keeps found chapters on redownload); verify green and `it_should_round_trip_chapters_none_empty_and_some` unchanged in assertions
- [ ] 2.3 Add `DownloadedQuality`, replacing `downloaded_height` + `quality_checks` (height overwritten on download, old-video rule moved in); verify green
- [ ] 2.4 Delete `late_data_checks.rs`; repository decodes each with `decode_late_step`; verify green
- [ ] 2.5 Commit "Give subtitles, chapters and quality their own types"

## 3. Thumbnail, summary and categorization reads

- [ ] 3.1 Add `Thumbnail`, replacing `thumbnail` + `thumbnail_filename`; move `thumbnail_basename`; `mark_downloaded`/`relocate_to_episode`/`reset_for_redownload` delegate; verify green
- [ ] 3.2 Delete `is_categorizable`, `is_categorization_pending`, `is_categorization_failed`, `next_categorization_at`; callers read `video.summary` / `video.categorization`; verify green
- [ ] 3.3 Commit "Give the thumbnail its own type"

## 4. Download

- [ ] 4.1 (human) Run the pre-flight query on the NAS database and confirm it returns 0: `SELECT count(*) FROM videos WHERE (status != 'DOWNLOADED' AND (filename IS NOT NULL OR quality IS NOT NULL OR duration_seconds IS NOT NULL)) OR (status = 'DOWNLOADED' AND (filename IS NULL OR quality IS NULL));` — if not 0, stop and revisit `Download.file` via /opsx:update
- [ ] 4.2 Add `DownloadedFile` and `Download` (`StepState` + `started` + `failure` + `file`); replace `status`, `download_attempts`, `download_failure`, `filename`, `quality`, `duration_seconds`; delete `is_download_due`/`next_download_at` logic from `Video`; move `is_downloaded`, `is_in_legacy_layout`; `VideoStatus` derived via `Download::status()`; verify green
- [ ] 4.3 Add the `download` value-object tests from design.md's Test Plan and `it_should_round_trip_every_download_state` in the repository; verify they pass
- [ ] 4.4 `StepResponse::download` and `VideoResponse` read the new paths; verify every `src/application/http/**` test passes unchanged in assertions
- [ ] 4.5 Commit "Make the download a step like the others"

## 5. Verification

- [ ] 5.1 Run `scripts/run-tests.sh --only e2e` once; verify it passes untouched (API did not move)
- [ ] 5.2 Run /audit-change and address its findings
- [ ] 5.3 Open one PR (what and why, field table, mermaid diagrams from design.md; no attribution lines, no model names) and verify GitHub CI is green; for a flaky e2e push an empty "Re-run CI" commit

## Workflow follow-up

- Archive the change after the PR merges.
