## 1. Domain: recent videos read model

- [x] 1.1 Add `VideoSearcher::list_recent(limit: usize)`, reusing the existing `PlaylistRepository::list()` / `PlaylistVideoRepository::list_for_playlist()` and `ChannelRepository::list()` / `ChannelVideoRepository::list_for_channel()` methods (no new repository trait methods needed) to gather every `(source, video)` pair, filter to `VideoStatus::Downloaded`, sort by `video.created_at` descending, and truncate to `limit`. Verify with unit tests covering: mixed playlist+channel results combined and sorted correctly, non-downloaded videos excluded, a video tracked by two sources appearing twice (once per source), and an empty result when nothing is downloaded.
- [x] 1.2 Introduce a small return type (e.g. a `RecentVideo` value carrying the `Video` plus its source kind and identifier) so the HTTP layer doesn't need to re-derive source attribution. Verify it compiles and is covered by the 1.1 tests.

## 2. HTTP: recent videos endpoint

- [x] 2.1 Add `VideoResponse`-sibling DTO (e.g. `RecentVideoResponse`) with `id`, `title`, and `source: { kind, id }`, per `openspec/changes/add-recent-videos-feed/specs/video-listing/spec.md`. Verify with a unit test on the `From` conversion.
- [x] 2.2 Add `list_recent_videos` handler parsing an optional `limit` query parameter (default 20, capped at 100), calling `VideoSearcher::list_recent`, and mapping to `RecentVideoResponse`. Verify with handler tests covering: default limit, explicit limit narrows results, limit above 100 is capped, empty list returns 200, and videos from both playlist and channel sources appear correctly tagged.
- [x] 2.3 Register `GET /videos/recent` in `src/application/http/mod.rs::api_router`. Verify the route responds via an integration-style test hitting the router directly (matching the existing pattern in `src/application/http/videos/mod.rs` tests).

## 3. SPA: Home tab

- [x] 3.1 Add `fetchRecentVideos()` to `web/src/api.js` calling `GET /videos/recent`. Verify by running the dev server and confirming the call succeeds against a daemon with synced videos.
- [x] 3.2 Add a `Home` tab component rendering title-only video cards from `fetchRecentVideos()`, using the existing polling pattern (`usePolling`). Verify by checking the tab renders the list and an empty/loading state, matching the style of `PlaylistList`/`ChannelList`.
- [x] 3.3 Make `Home` the default/first tab in `App.jsx`'s tab navigation. Verify by loading the SPA root and confirming Home is shown first.
- [x] 3.4 Add deep-link support: clicking a Home card switches to the Playlists or Channels tab (by `source.kind`), resolves the full playlist/channel object via the existing `fetchPlaylists()`/`fetchChannels()` list calls matched by `source.id`, and passes an initial video id down so `PlaylistDetail`/`ChannelDetail` preselect and autoplay the matching video from their already-fetched video list. Verify manually in the browser: click a Home card and confirm it lands on the right playlist/channel with the video already selected and playing.

## 4. Verification

- [x] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; all must pass.
- [x] 4.2 Manually exercise the full flow in a browser against a running daemon with at least one downloaded video in a playlist and one in a channel: Home tab lists both, newest first, and clicking each navigates to the correct source with the video autoplaying.
