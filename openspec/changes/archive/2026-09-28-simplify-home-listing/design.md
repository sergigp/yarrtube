## Files

- `src/domain/video/sourced_video.rs` (renamed from `recent_video.rs`): `SourcedVideo`, a video paired with the source that tracks it, used inside the searcher to apply the section rules. `VideoSource` stays here.
- `src/domain/video/home_video_view.rs`: new `HomeVideoView`, the flat card read model, built from a picked `SourcedVideo`.
- `src/domain/video/home_videos.rs`: `HomeVideos` holds `HomeVideoView`s.
- `src/domain/video/mod.rs`: module and export renames.
- `src/domain/services/video_searcher.rs`: renames `list` to `list_for_playlist` (matching `list_for_channel`); drops `list_recent`, `list_continue_watching` and `list_quick_watches` (and `list_recent`'s helpers `recent_from_*`, renamed `downloaded_from_*`); `list_home` maps the picked videos to `HomeVideoView`.
- `src/application/http/videos/mod.rs`: `list_videos_for_playlist` calls `list_for_playlist`; drops `list_recent_videos`, `list_continue_watching_videos`, `list_quick_watch_videos`, `list_across_sources`, `ListRecentVideosQuery` and the recent limit constants. The section rule tests move onto `list_home_videos`.
- `src/application/http/videos/dto.rs`: `RecentVideoResponse` → `HomeVideoResponse`, `RecentVideoSourceResponse` → `HomeVideoSourceResponse`, both mapped from `HomeVideoView`. JSON unchanged.
- `src/application/http/mod.rs`: drops the `/videos/recent`, `/videos/continue-watching` and `/videos/quick-watches` routes.

## Types & Signatures

```rust
// src/domain/video/sourced_video.rs
pub struct SourcedVideo {
    pub video: Video,
    pub source: VideoSource,
}
```

```rust
// src/domain/video/home_video_view.rs
pub struct HomeVideoView {
    pub youtube_id: VideoId,
    pub title: String,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub watched: bool,
    pub position_seconds: i64,
    pub source: VideoSource,
}

impl From<SourcedVideo> for HomeVideoView;

// src/domain/video/home_videos.rs
pub struct HomeVideos {
    pub continue_watching: Vec<HomeVideoView>,
    pub quick_watches: Vec<HomeVideoView>,
    pub latest: Vec<HomeVideoView>,
}
```

```rust
// src/domain/services/video_searcher.rs
pub trait VideoSearcherApi: Send + Sync {
    fn list_for_playlist(&self, playlist_id: &PlaylistId) -> Result<Vec<VideoView>, ListVideosError>; // renamed from `list`
    fn list_for_channel(&self, channel_id: &ChannelHandle) -> Result<Vec<VideoView>, ListVideosError>;
    fn list_home(&self, limits: HomeLimits) -> Result<HomeVideos, ListVideosError>;
}

impl VideoSearcher {
    // Section rules keep working on `SourcedVideo` (they need the entity);
    // only the picked videos become `HomeVideoView`s.
    fn downloaded_across_sources(&self) -> Result<Vec<SourcedVideo>, ListVideosError>;
    fn downloaded_from_channels(&self) -> Result<Vec<SourcedVideo>, ListVideosError>;
    fn downloaded_from_playlists(&self) -> Result<Vec<SourcedVideo>, ListVideosError>;
    fn continue_watching_in(videos: &[SourcedVideo], now: DateTime<Utc>, limit: usize) -> Vec<SourcedVideo>;
    fn quick_watches_in(videos: &[SourcedVideo], limit: usize) -> Vec<SourcedVideo>;
    fn latest_in(videos: Vec<SourcedVideo>, limit: usize) -> Vec<SourcedVideo>;
    // pick_once, not_shown_in, once_per_youtube_video: unchanged apart from SourcedVideo
}
```

```rust
// src/application/http/videos/dto.rs
pub struct HomeVideoSourceResponse { /* fields of RecentVideoSourceResponse */ }
pub struct HomeVideoResponse { /* fields of RecentVideoResponse */ }
impl From<HomeVideoView> for HomeVideoResponse;

pub struct HomeResponse {
    pub continue_watching: Vec<HomeVideoResponse>,
    pub quick_watches: Vec<HomeVideoResponse>,
    pub latest: Vec<HomeVideoResponse>,
}
```

## Call Stack

**Home** (only the output types change):
`GET /videos/home` → `list_home_videos` → `VideoSearcher::list_home(HomeLimits { 6, 6, 18 })` → `downloaded_across_sources()` (`downloaded_from_channels` then `downloaded_from_playlists`, each `VideoRepository::find_many` per source) → `continue_watching_in` → `not_shown_in` → `quick_watches_in` → `not_shown_in` → `latest_in` → each section `.map(HomeVideoView::from)` → `HomeResponse::from(HomeVideos)` (cards via `HomeVideoResponse::from`)

## Test Plan

All in `src/application/http/videos/mod.rs`, through `list_home_videos`. The moved tests pin behaviour that already exists, so they pass when added; they are the safety net for removing the endpoints and reshaping the types.

### 1. Behaviour (acceptance)

Kept as they are: the six existing home tests (`it_should_list_a_downloaded_video_under_latest_on_home` … `it_should_cap_latest_on_home_at_18`).

Moved from `/videos/recent`:
1. `it_should_list_no_home_videos_if_nothing_downloaded`: three empty sections.
2. `it_should_list_videos_from_playlists_and_channels_on_home`: both sources under `latest`, each with its source.
3. `it_should_include_the_channel_source_on_home`: channel name and avatar filename.
4. `it_should_include_the_playlist_source_on_home`: playlist name, no avatar.
5. `it_should_include_whether_a_video_was_watched_on_home`: a watched video under `latest` with `watched: true`.
6. `it_should_list_a_latest_video_once_per_source_on_home`
7. `it_should_exclude_not_downloaded_videos_from_home`: a pending in-progress video and a pending short video appear nowhere.

Moved from `/videos/continue-watching` (a rejected video shows under `latest`):
8. `it_should_order_continue_watching_by_last_played_first_on_home`
9. `it_should_continue_only_videos_played_within_a_week_on_home`: exactly 7 days under `continue_watching`, 7 days + 1s under `latest`.
10. `it_should_continue_only_videos_started_past_30_seconds_on_home`: 31s under `continue_watching`, 30s under `latest`.
11. `it_should_not_continue_watched_or_never_played_videos_on_home`: both under `latest`.
12. `it_should_list_a_continue_watching_video_once_from_the_channel_on_home`: channel copy only, even when the playlist copy was played later; neither copy under `latest`.

Moved from `/videos/quick-watches`:
13. `it_should_order_quick_watches_newest_first_on_home`
14. `it_should_list_only_videos_under_15_minutes_as_quick_watches_on_home`: 899s under `quick_watches`, 900s under `latest`.
15. `it_should_not_list_videos_without_duration_as_quick_watches_on_home`: under `latest`.
16. `it_should_not_list_watched_videos_as_quick_watches_on_home`: under `latest`, `watched: true`.
17. `it_should_list_a_quick_watch_once_from_the_channel_on_home`: channel copy only, even when the playlist copy is newer.

Removed with their endpoints: every `list_recent` / `continue_watching` / `quick_watches` handler test not listed above, including the `limit` query tests (default, explicit, cap).

### 2. Infrastructure

None (no adapter changes).
