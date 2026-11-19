# Tasks

## 1. Walking skeleton

- [x] 1.1 Add every type and signature from design.md's Types & Signatures: `PlaybackPosition::is_started(duration_seconds)` (body ignores the duration, keeps `> 30s`), `WatchState::progress(duration)` / `is_continuable(duration, now)` (rename of `is_in_progress`), `WatchProgress::is_in_progress`, `Video::watch_progress` / `Video::is_continuable`, `in_progress` on `HomeVideoView`, `VideoResponse`, `HomeVideoResponse`, `CategoryVideoResponse` (wired from `watch_progress().is_in_progress()`), and `in_progress` on the web types, fixture builders and `WatchableVideo`; update callers (`VideoSearcher`, `PlexWatchSyncer`, `PlexWatchState`) and existing test expectations for the new field. Done when `cargo build` and `scripts/run-tests.sh --lint --web --rust` pass

## 2. Behaviour (TDD)

- [ ] 2.1 `it_should_continue_only_videos_started_past_20_percent_on_home`: 40-min videos at 420s / 480s split between latest and continue watching
- [ ] 2.2 `it_should_continue_a_long_video_played_for_10_minutes_on_home`: 2-hour video at 600s continued
- [ ] 2.3 Keep `it_should_continue_only_videos_started_past_30_seconds_on_home` as the no-duration regression (verify it still passes)
- [ ] 2.4 `it_should_report_whether_listed_channel_videos_are_in_progress`: 480s → true, 300s → false, watched → false; positions reported as saved
- [ ] 2.5 `it_should_report_an_old_started_video_in_progress_when_listing_channel_videos`: last played 10 days ago still `in_progress: true`
- [ ] 2.6 `it_should_report_in_progress_on_home_videos`: continue watching entry reports `in_progress: true`
- [ ] 2.7 `it_should_report_in_progress_on_category_videos`: 40-min category video at 480s reports `in_progress: true`
- [ ] 2.8 `it_should_save_a_position_under_the_started_threshold`: unwatched 40-min, report 300s → unwatched, position 300
- [ ] 2.9 `it_should_keep_a_watched_video_watched_early_in_a_rewatch`: watched 40-min, report 420s → stays watched at 0
- [ ] 2.10 `it_should_mark_a_watched_video_unwatched_once_a_rewatch_counts_as_started` (replaces `…_past_10_percent_of_a_rewatch`; drop `REWATCH_RESET_THRESHOLD`): watched 40-min, report 600s → unwatched at 600
- [ ] 2.11 `it_should_not_push_a_position_under_20_percent`: local 40-min at 300s → no Plex mutation, agreed Unplayed
- [ ] 2.12 `it_should_push_a_position_past_20_percent`: local 40-min at 480s → Plex position 480
- [ ] 2.13 `it_should_not_pull_a_plex_position_under_20_percent`: 40-min episode offset 300s → video stays unwatched at 0
- [ ] 2.14 `it_should_count_a_rewatch_in_plex_under_20_percent_as_still_watched`: watched 40-min episode offset 420s → stays watched; remove `is_short_rewatch`
- [ ] 2.15 `sourceVideos.test.ts` — `isInProgress` follows `in_progress`, not `position_seconds > 0`; update `playback_position.rs` unit tests for the duration-aware rule

## 3. Infrastructure adapters (TDD)

(none: no adapter changes)

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --lint --web --rust`; all pass
- [ ] 4.2 Manual (agent): `scripts/run-local.sh --port <free port> --fresh-videos`, record progress via the HTTP API on a downloaded video under and over 20% of its duration, check `GET /api/videos/home` continue watching and the source page "In progress" filter in the Playwright browser; stop the server and delete `yarrtube.sqlite3` and the fresh videos dir
- [ ] 4.3 Manual (human): with Plex configured, stop an episode under 20% and another past 20% in a Plex client; after a watch sync only the second shows under "Continue watching"

## Workflow follow-up

- `/openspec:finish` to sync specs, archive and take the PR out of draft.
