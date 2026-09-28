## 1. Walking skeleton

- [x] 1.1 Reshape the types and remove the three listings, per design.md (## Files, ## Types & Signatures):
  - rename `RecentVideo` to `SourcedVideo` (`sourced_video.rs`) and `recent_from_*` to `downloaded_from_*`
  - add `HomeVideoView` (`home_video_view.rs`) with `From<SourcedVideo>`; this mapping is real, not a stub, because it reshapes card fields the API already returns and the six existing home tests pin them
  - `HomeVideos` holds `HomeVideoView`s
  - rename the DTOs to `HomeVideoResponse` / `HomeVideoSourceResponse`, mapped from `HomeVideoView`
  - rename `VideoSearcherApi::list` to `list_for_playlist`, matching `list_for_channel`
  - remove `list_recent`, `list_continue_watching` and `list_quick_watches` from `VideoSearcherApi`, together with their handlers, routes, `ListRecentVideosQuery`, the recent limit constants, `list_across_sources` and their tests

  Done when `cargo build` succeeds and the remaining tests pass. No new tests.

## 2. Behaviour (TDD)

Each test here moves an existing rule onto `/videos/home`. It pins behaviour that already exists, so it is expected to pass on its first run; there is no red step.

- [x] 2.1 `it_should_list_no_home_videos_if_nothing_downloaded`: three empty sections
- [x] 2.2 `it_should_list_videos_from_playlists_and_channels_on_home`: both sources under `latest`, each with its own source
- [ ] 2.3 `it_should_include_the_channel_source_on_home`: channel name and avatar filename
- [ ] 2.4 `it_should_include_the_playlist_source_on_home`: playlist name, no avatar
- [ ] 2.5 `it_should_include_whether_a_video_was_watched_on_home`: a watched video under `latest` with `watched: true`
- [ ] 2.6 `it_should_list_a_latest_video_once_per_source_on_home`
- [ ] 2.7 `it_should_exclude_not_downloaded_videos_from_home`: a pending in-progress video and a pending short video appear nowhere
- [ ] 2.8 `it_should_order_continue_watching_by_last_played_first_on_home`
- [ ] 2.9 `it_should_continue_only_videos_played_within_a_week_on_home`: exactly 7 days under `continue_watching`, 7 days + 1s under `latest`
- [ ] 2.10 `it_should_continue_only_videos_started_past_30_seconds_on_home`: 31s under `continue_watching`, 30s under `latest`
- [ ] 2.11 `it_should_not_continue_watched_or_never_played_videos_on_home`: both under `latest`
- [ ] 2.12 `it_should_list_a_continue_watching_video_once_from_the_channel_on_home`: channel copy only, even when the playlist copy was played later
- [ ] 2.13 `it_should_order_quick_watches_newest_first_on_home`
- [ ] 2.14 `it_should_list_only_videos_under_15_minutes_as_quick_watches_on_home`: 899s under `quick_watches`, 900s under `latest`
- [ ] 2.15 `it_should_not_list_videos_without_duration_as_quick_watches_on_home`: under `latest`
- [ ] 2.16 `it_should_not_list_watched_videos_as_quick_watches_on_home`: under `latest`, `watched: true`
- [ ] 2.17 `it_should_list_a_quick_watch_once_from_the_channel_on_home`: channel copy only, even when the playlist copy is newer

## 3. Infrastructure adapters (TDD)

_None: no adapter changes._

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass
- [ ] 4.2 `npm run lint` and `npm run build` in `web/` pass, and `./scripts/run-smoke-tests.sh` passes (the home view only calls `/videos/home`)
