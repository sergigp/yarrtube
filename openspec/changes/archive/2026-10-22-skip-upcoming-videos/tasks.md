## 1. Walking skeleton

- [x] 1.1 Add `live_status` + `FlatPlaylistEntry::is_watchable` (returns `true`) in `ytdlp.rs`; in the playlist items adapter switch `base_url` to the API root (`/playlistItems` appended), add `VideosResponse`/`VideoItem`/`VideoSnippet`, `is_broadcast_pending` (returns `false`) and `find_not_yet_watchable` (returns empty set), update existing tests' mock paths to `/playlistItems`. Done when `cargo build` and `cargo test --locked` pass.

## 2. Behaviour (TDD)

- [x] 2.1 None: reconciler behaviour is unchanged (see design.md Test Plan). Verify by checking that the existing channel/playlist reconciler add and evict tests still pass.

## 3. Infrastructure adapters (TDD)

### ytdlp::list_channel_videos
- [x] 3.1 `it_should_skip_upcoming_entries_when_listing_channel_videos`: implement `is_watchable` + filter; test passes
- [x] 3.2 `it_should_skip_live_and_post_live_entries_when_listing_channel_videos`: test passes
- [x] 3.3 `it_should_keep_entries_with_a_finished_or_absent_live_status`: test passes

### YoutubeApiPlaylistItemsRepository
- [x] 3.4 `it_should_skip_items_reported_as_upcoming`: call `find_not_yet_watchable` per page and drop returned ids; add `/videos` mocks (empty items) to existing tests; test passes
- [x] 3.5 `it_should_skip_items_reported_as_live`: test passes
- [x] 3.6 `it_should_request_live_status_for_the_page_video_ids`: test passes
- [x] 3.7 `it_should_keep_items_missing_from_the_videos_response`: test passes
- [x] 3.8 `it_should_fail_when_the_live_status_lookup_fails`: error logged like the existing `playlistItems` failures; test passes

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass
- [x] 4.2 `scripts/run-smoke-tests.sh` passes
- [x] 4.3 Manual (agent): run real `yt-dlp --flat-playlist --print-json` against a channel with a scheduled premiere and confirm the `live_status` values match what is filtered; for a playlist, confirm `videos.list` returns `liveBroadcastContent: "upcoming"` for a scheduled premiere
