## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures) across all layers: migration `0005_last_played_at.sql` registered in `sqlite_migrations.rs`; `Video.last_played_at` (set to `None` in `create`, read and written by `SqliteVideoRepository`); `Video::is_in_progress` / `is_quick_watch` returning `false`; `VideoSearcher::new` taking a `Clock` (wired in `serve.rs` and every existing test constructor); `list_continue_watching` / `list_quick_watches` returning `Ok(vec![])`; the two handlers routed at `/videos/continue-watching` and `/videos/quick-watches`; `RecentVideoResponse.position_seconds` (existing expected values updated); `fetchContinueWatchingVideos` / `fetchQuickWatchVideos` in `api.js`; `Home.jsx` split into three sections (new ones hidden when empty); `WatchProgressBar.jsx` rendering nothing. Done when `cargo build` succeeds and all existing tests pass. No new behaviour and no new tests.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_record_the_last_played_time_on_every_copy`: `update_watch_state` sets `last_played_at` to now on every copy
- [x] 2.2 `it_should_record_the_last_played_time_even_if_the_watch_state_is_unchanged`: the watched ≤10% branch still sets `last_played_at`
- [x] 2.2b Existing `it_should_mark_the_video_watched_at_90_percent` and `it_should_mark_a_watched_video_unwatched_past_10_percent_of_a_rewatch` expect `last_played_at`: the two branches that change the watch state also set it, so every branch of `update_watch_state` does
- [ ] 2.3 `it_should_not_change_the_last_played_time_when_marking_a_channel_watched`: `mark_watched` leaves `last_played_at` untouched
- [ ] 2.4 `it_should_list_no_continue_watching_videos_if_none_in_progress`: empty listing when nothing is in progress
- [ ] 2.5 `it_should_list_a_recently_started_video_in_continue_watching`: `list_continue_watching` collects across sources, filters on `is_in_progress`, and the response carries `position_seconds`
- [ ] 2.6 `it_should_order_continue_watching_by_last_played_first`: sort by `last_played_at` desc
- [ ] 2.7 `it_should_exclude_videos_last_played_over_a_week_ago_from_continue_watching`: 7-day window against the clock (boundary: exactly 7 days is included)
- [ ] 2.8 `it_should_exclude_barely_started_videos_from_continue_watching`: position must be > 30s
- [ ] 2.9 `it_should_exclude_watched_and_never_played_videos_from_continue_watching`: unwatched and `last_played_at` present
- [ ] 2.10 `it_should_exclude_not_downloaded_videos_from_continue_watching`: only `Downloaded` videos
- [ ] 2.11 `it_should_list_a_continue_watching_video_once_across_sources`: `once_per_youtube_video` keeps the channel copy
- [ ] 2.12 `it_should_honor_and_cap_the_continue_watching_limit`: default 20, explicit N, capped at 100
- [ ] 2.13 `it_should_list_no_quick_watches_if_none_short`: empty listing when no short videos
- [ ] 2.14 `it_should_list_short_unwatched_videos_as_quick_watches_newest_first`: `list_quick_watches` filters on `is_quick_watch`, sorted by `created_at` desc
- [ ] 2.15 `it_should_exclude_videos_of_15_minutes_or_more_from_quick_watches`: duration < 900s (boundary: 899 in, 900 out)
- [ ] 2.16 `it_should_exclude_videos_without_duration_from_quick_watches`: unknown duration is not quick
- [ ] 2.17 `it_should_exclude_watched_and_not_downloaded_videos_from_quick_watches`: unwatched and `Downloaded` only
- [ ] 2.18 `it_should_list_a_quick_watch_once_across_sources`: dedupe applied to quick watches
- [ ] 2.19 `it_should_honor_and_cap_the_quick_watches_limit`: default 20, explicit N, capped at 100

## 3. Infrastructure adapters (TDD)

`SqliteVideoRepository`:
- [ ] 3.1 `it_should_round_trip_a_video_with_a_last_played_time`: the column is written by `save`/`update` and read back
- [ ] 3.2 `it_should_round_trip_a_video_never_played`: NULL maps to `None`

Migrations:
- [ ] 3.3 `it_should_backfill_the_last_played_time_of_part_watched_videos_when_migrating`: migration `0005` backfill touches only unwatched videos with a position

## 4. Verification

- [ ] 4.1 Web UI: `WatchProgressBar` renders position / duration on continue-watching cards and new sections are hidden when empty. Verify with `npm run lint` and `npm run build` in `web/`.
- [ ] 4.2 Extend `smoke-tests/tests/channel.spec.js`: after the existing part-playback, navigate home and assert the video appears under "Continue watching" (this exercises `/videos/continue-watching`). Verify with `./scripts/run-smoke-tests.sh`.
- [ ] 4.3 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass
- [ ] 4.4 Manual check with `scripts/run-local.sh` against a copy of the prod DB: the backfilled part-watched video ("How to set up Herdr…") shows under "Continue watching" with a progress bar, "Quick watches" lists videos under 15 min, and "Latest videos" is unchanged
