# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers: migration `0010_video_subtitles.sql`, `SubtitleState` + `Video` fields/transitions, `EpisodePaths::subtitle`, `subtitle_staging_dir`, the `video_transcript` aggregate, `SubtitleFetcher`, `Task::FetchSubtitles` + `TaskLane::Subtitles` + `DeleteVideoFile.subtitle_filename`, `FetchSubtitlesTask` registered in `tasks::registry`, `VideoTranscriptRepository` + SQLite impl, `VideoRepository::update_subtitles`, `VideoDownloaderRepository::{probe_captions, fetch_subtitles}` (yt-dlp impl + fake), wired in `serve.rs` and into `InternalVideoReconciler` and the playlist/channel reconcilers. Bodies return trivial values (`Ok(())`, `None`, empty `CaptionTracks`); never `todo!()`. Done when `cargo build` succeeds and all existing tests pass

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_fetch_the_original_language_subtitles`: places `<stem>.en.srt`, records Fetched + filename, stores the transcript
- [x] 2.2 `it_should_record_auto_generated_captions_as_automatic`
- [x] 2.3 `it_should_mark_an_old_video_without_captions_unavailable`
- [x] 2.4 `it_should_keep_a_recent_video_without_captions_missing`
- [x] 2.5 `it_should_fall_back_to_the_creation_time_when_no_publish_time_is_reported`
- [x] 2.6 `it_should_leave_subtitles_missing_when_the_probe_fails`
- [x] 2.7 `it_should_leave_subtitles_missing_when_the_download_fails`
- [x] 2.8 `it_should_skip_if_subtitles_already_fetched`
- [x] 2.9 `it_should_skip_a_video_that_is_not_downloaded`
- [x] 2.10 `it_should_skip_if_video_no_longer_exists`
- [x] 2.11 `it_should_keep_download_progress_while_recording_subtitles`
- [x] 2.12 `it_should_discard_the_staging_dir`
- [x] 2.13 `it_should_schedule_a_subtitle_fetch_for_a_downloaded_video_missing_subtitles` (channel reconcile)
- [x] 2.14 `it_should_not_schedule_a_subtitle_fetch_for_fetched_or_unavailable_subtitles`
- [x] 2.15 `it_should_not_schedule_a_subtitle_fetch_for_a_video_not_downloaded`
- [x] 2.16 `it_should_not_schedule_a_second_subtitle_fetch_if_one_is_queued`
- [x] 2.17 `it_should_not_schedule_a_subtitle_fetch_for_a_video_being_redownloaded`
- [x] 2.18 `it_should_keep_recorded_subtitle_files`: orphan sweep keeps recorded `.srt` and deletes a stray one
- [x] 2.19 `it_should_delete_the_transcript_of_a_removed_video` (channel reconcile)
- [x] 2.20 `it_should_schedule_a_subtitle_fetch_for_a_downloaded_video_missing_subtitles` (playlist reconcile, wiring)
- [x] 2.21 `it_should_delete_the_transcript_of_a_removed_video` (playlist reconcile)
- [x] 2.22 `it_should_schedule_the_deletion_of_the_subtitle_file` in both delete-video-file subscribers
- [x] 2.23 `it_should_delete_an_episodes_subtitle_file` (delete video file task)
- [x] 2.24 `it_should_decode_a_payload_queued_without_a_subtitle_filename`
- [x] 2.25 `it_should_list_a_fetch_subtitles_task_with_video_and_playlist_names` (tasks HTTP)
- [x] 2.26 `CaptionTracks::original` value tests (design Test Plan item 26) pass
- [x] 2.27 `TranscriptText::from_srt` tests (item 27) pass
- [x] 2.28 `SubtitleState`, `CaptionLanguage`, `EpisodePaths::subtitle` tests (item 28) pass
- [x] 2.29 `web/src/lib/tasks.ts` describes and categorizes `fetch_subtitles`; verify with `npm run check`
- [x] 2.30 Document subtitle files and the post-upgrade backfill in `doc/PLEX.md` and `doc/ARCHITECTURE.md`

## 3. Infrastructure adapters (TDD)

yt-dlp (`infrastructure/shared/ytdlp.rs`):
- [x] 3.1 Run real `yt-dlp -J --skip-download` against 3–4 videos (uploaded captions, auto-only, non-English, no speech); save trimmed info-JSON fixtures; confirm `language` and `-orig` behave as `CaptionTracks::original` assumes (adjust 2.26 if not)
- [x] 3.2 `it_should_parse_caption_tracks_and_language_from_the_info_json`
- [x] 3.3 `it_should_parse_the_publish_timestamp_from_the_info_json`
- [x] 3.4 `it_should_fail_the_probe_on_a_clean_failed_exit`
- [x] 3.5 `it_should_pass_manual_subtitle_flags_for_a_manual_track` and `it_should_pass_auto_subtitle_flags_for_an_automatic_track`
- [x] 3.6 `it_should_return_the_written_srt_and_its_contents`
- [x] 3.7 `it_should_fail_when_no_srt_was_written`

SQLite transcript repository:
- [x] 3.8 `it_should_replace_the_transcript_of_a_video_on_save`
- [ ] 3.9 `it_should_delete_a_video_transcript`

SQLite video repository:
- [ ] 3.10 `it_should_update_only_the_subtitle_columns`
- [ ] 3.11 `it_should_default_existing_videos_to_missing_subtitles`

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `npm run check` in `web/` all pass
- [ ] 4.2 Run `scripts/run-local.sh` on a library with downloaded videos: after one reconcile, subtitle fetches run one at a time; `.srt` files appear next to episodes; captionless old videos end Unavailable; tasks view shows "Fetching subtitles of …"
- [ ] 4.3 Run `scripts/run-smoke-test.sh` and verify it passes
