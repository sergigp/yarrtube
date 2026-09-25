## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md (## Files, ## Types & Signatures), wired end-to-end:
  - migration `0002` registered in `sqlite_migrations.rs`
  - `PlaybackPosition`, `ChannelView` and `UpdateWatchStateError`
  - `Video` fields and transitions
  - `VideoRepository::find_by_youtube_id`, plus the new columns in the SQLite repository's SQL and row mapping
  - `VideoWatchStateUpdater`, and the new `ChannelSearcher` constructor and return type (flat `ChannelView`)
  - DTO fields, `ChannelListItemResponse` and `RecordProgressRequest`
  - the two handlers and routes, `ApiServices.video_watch_state_updater`, and the `serve.rs` wiring
  - SPA API functions, `useWatchProgress`, `WatchedTick`, and the new controls rendered in the Sidebar, ChannelDetail, PlaylistDetail and Home

  Bodies return trivial values: transitions return `self` unchanged, `find_by_youtube_id` returns an empty `Vec`, service methods return `Ok(())`, `unwatched_count` is `0`, and SPA handlers are no-ops. Update existing `Video { .. }` literals and the `ChannelSearcher::new` call sites (existing list tests expect the flat `ChannelListItemResponse` with `unwatched_count: 0`). Done when `cargo build` succeeds, `cargo test --locked` passes, and `npm run build` and `npm run lint` succeed in `web/`. No new tests and no new behaviour.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_record_playback_progress`: drives saving the position of an unwatched video (`VideoWatchStateUpdater::record_progress`, `find_by_youtube_id`, `Video::record_progress`).
- [x] 2.2 `it_should_mark_the_video_watched_at_90_percent`: drives the 90% watched rule and resetting the position to 0.
- [x] 2.3 `it_should_use_the_reported_duration_if_none_is_recorded`: drives the fallback to the player-reported duration.
- [x] 2.4 `it_should_only_record_the_position_if_duration_is_unknown`: drives the unknown-duration branch.
- [x] 2.5 `it_should_keep_a_watched_video_watched_early_in_a_rewatch`: drives leaving a watched video unchanged at or below 10%.
- [x] 2.6 `it_should_mark_a_watched_video_unwatched_past_10_percent_of_a_rewatch`: drives the rewatch reset.
- [x] 2.7 `it_should_record_progress_on_every_copy_of_the_video`: drives applying progress to every stored copy of a YouTube video.
- [x] 2.8 `it_should_fail_to_record_progress_of_an_unknown_video`: drives `UpdateWatchStateError::VideoNotFound` mapping to 400.
- [x] 2.9 `it_should_fail_to_record_progress_if_position_missing`: drives `required(.., MISSING_POSITION)`.
- [x] 2.10 `it_should_fail_to_record_progress_if_invalid_position_provided`: drives the `PlaybackPosition` `ValidationError` mapping to 400.
- [x] 2.11 `it_should_mark_every_downloaded_channel_video_watched`: drives `mark_channel_watched` (downloaded videos only, including their playlist copies, via `Video::mark_watched`).
- [x] 2.12 `it_should_fail_to_mark_watched_a_missing_channel`: drives `UpdateWatchStateError::ChannelNotFound` mapping to 400.
- [x] 2.13 `it_should_fail_to_mark_watched_if_invalid_handle_provided`: drives the `ChannelHandle` validation mapping on the new route.
- [x] 2.14 `it_should_count_unwatched_downloaded_videos_when_listing_channels`: drives `ChannelSearcher` computing `unwatched_count` and the flat channel list mapping.
- [x] 2.15 `it_should_include_watch_state_when_listing_channel_videos`: drives the `watched`/`position_seconds` mapping on `VideoResponse`.
- [x] 2.16 `it_should_include_watch_state_when_listing_playlist_videos`: drives the same fields through the playlist listing.
- [x] 2.17 `it_should_include_whether_recent_videos_were_watched`: drives `watched` on `RecentVideoResponse`.
- [x] 2.18 `it_should_keep_watch_state_when_redownloading_a_missing_file` (in `reconcile_channel_task.rs`): a regression guard for `reset_for_redownload`. It may pass on its first run because struct update already keeps the fields. If it does, ask whether to keep it and skip the cycle.
- [x] 2.19 `it_should_accept_zero_and_positive_positions` (in `playback_position.rs`): drives `PlaybackPosition::new` accepting positions of 0 and above.
- [x] 2.20 `it_should_reject_a_negative_position` (in `playback_position.rs`): drives the exact negative-position message.
- [x] 2.21 `it_should_keep_a_watched_video_watched_when_playing_on_past_90_percent`: drives keeping a watched video unchanged at or above 90%.

## 3. Infrastructure adapters (TDD)

SQLite migrations:
- [x] 3.1 `it_should_default_existing_videos_to_unwatched_when_migrating`: drives the `0002` SQL. Existing rows read back unwatched with position 0.

`SqliteVideoRepository`:
- [x] 3.2 `it_should_round_trip_a_watched_video_with_a_playback_position`: drives the new columns in `save`/`find`/`row_to_video`.
- [x] 3.3 `it_should_find_every_copy_of_a_youtube_video`: drives the `find_by_youtube_id` query.
- [x] 3.4 `it_should_find_no_copies_of_an_unknown_youtube_video`: drives the empty result.

## 4. Verification

- [x] 4.1 Implement the SPA behaviour on top of the skeleton, following the web-ui delta spec:
  - `useWatchProgress`: resume, a 15s throttle, and reports on pause, ended and cleanup, with a beacon on `pagehide`.
  - `WatchedTick` on thumbnails in the channel, playlist and home views.
  - the sidebar badge and mark-watched row action.
  - "Mark all watched" in the channel view.
  - `ChannelDetail`/`PlaylistDetail` keep the selected video by id.

  Verify `npm run build` and `npm run lint` succeed in `web/`.
- [x] 4.2 Extend `smoke-tests/tests/channel.spec.js` (plus a `markItemWatched` helper in `smoke-tests/helpers/sidebar.js`) so every new route the UI calls is exercised:
  - after the video plays and is paused, reloading resumes past 0 (progress).
  - the sidebar mark-watched action removes the channel badge (channel watched).

  Verify `scripts/run-smoke-tests.sh` passes.
- [x] 4.3 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked`. Verify all three succeed.
- [x] 4.4 Run the app against an existing `yarrtube.sqlite3` with `scripts/run-local.sh` and check:
  - the migration applies and existing videos show as unwatched, with channel badges showing their downloaded counts.
  - watching past 90% ticks the video and lowers the badge within one poll.
  - closing the tab mid-video and reopening resumes near the same position.

## 5. PR review

- [x] 5.1 Rename `ChannelSearcher` to `ChannelViewSearcher` (`channel_view_searcher.rs`, `ChannelViewSearcherApi`, `ApiServices.channel_view_searcher`, drop the `search_all` doc comment), `VideoWatchStateUpdaterApi::record_progress` to `update`, and `Video::record_progress` to `update_watch_state`. Refactor only: the suite stays green.
- [x] 5.2 Remove `it_should_keep_watch_state_when_redownloading_a_missing_file`; strengthen `it_should_redownload_videos_with_missing_file` to seed a watched video with a recorded duration and assert an explicit expected video (cleared: status, quality, filename, thumbnail, duration; kept: everything else, including watch state).
- [x] 5.3 `it_should_accept_a_positive_duration` (in `video_duration.rs`): drives `VideoDuration::new` accepting positive durations.
- [ ] 5.4 `it_should_reject_a_non_positive_duration` (in `video_duration.rs`): drives the exact non-positive message.
- [ ] 5.5 `it_should_fail_to_record_progress_if_invalid_duration_provided`: drives parsing `duration_seconds` with `VideoDuration` in the handler and passing `Option<VideoDuration>` through `update`/`update_watch_state`. `it_should_only_record_the_position_if_duration_is_unknown` goes back to no duration anywhere; the domain keeps the non-positive filter for a recorded 0, with its comment saying so.
- [ ] 5.6 SPA: `useWatchProgress` reports `duration_seconds` only when the player's duration is at least 1s. Verify `npm run build` and `npm run lint`.
- [ ] 5.7 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `cargo test --locked` and `scripts/run-smoke-tests.sh`; push to the PR.
