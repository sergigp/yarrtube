## Files

- `migrations/0005_last_played_at.sql`: adds `videos.last_played_at` and backfills it from `updated_at` for unwatched videos with a saved position.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers migration `0005`.
- `src/domain/video/video.rs`: `last_played_at` field. `update_watch_state` sets it. New rules `is_in_progress` and `is_quick_watch` and their constants live on the entity.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: reads and writes the new column (`VIDEO_COLUMNS`, `save`, `update`, row mapping).
- `src/domain/services/video_searcher.rs`: `list_continue_watching` and `list_quick_watches`. Takes a `Clock` for the 7-day window. Shares the cross-source collection with `list_recent`.
- `src/application/http/videos/mod.rs`: two handlers, reusing `ListRecentVideosQuery` and the recent limits.
- `src/application/http/videos/dto.rs`: `RecentVideoResponse` gains `position_seconds`.
- `src/application/http/mod.rs`: routes `/videos/continue-watching` and `/videos/quick-watches`.
- `src/serve.rs`: passes the clock to `VideoSearcher::new`.
- `web/src/api.js`: `fetchContinueWatchingVideos` and `fetchQuickWatchVideos`.
- `web/src/components/Home.jsx`: renders three sections in order; the new ones are hidden when empty.
- `web/src/components/WatchProgressBar.jsx`: thin bar over a thumbnail showing position / duration.
- `smoke-tests/tests/channel.spec.js`: after the existing part-playback, the home view shows the video under "Continue watching".

## Types & Signatures

```sql
-- migrations/0005_last_played_at.sql
ALTER TABLE videos ADD COLUMN last_played_at TEXT;
UPDATE videos SET last_played_at = updated_at
 WHERE watched_at IS NULL AND playback_position_seconds > 0;
```

```rust
// src/domain/video/video.rs
const IN_PROGRESS_MIN_POSITION_SECONDS: i64 = 30;
const IN_PROGRESS_WINDOW: Duration = Duration::days(7);
const QUICK_WATCH_MAX_DURATION_SECONDS: i64 = 900;

pub struct Video {
    // ...existing fields
    pub last_played_at: Option<DateTime<Utc>>,
}

impl Video {
    // `create` sets `last_played_at: None`. `mark_watched` and
    // `reset_for_redownload` leave it unchanged.
    // Sets `last_played_at = Some(now)` in every branch, including the ones
    // that leave the watch state untouched.
    pub fn update_watch_state(self, position: PlaybackPosition, reported_duration: Option<VideoDuration>, now: DateTime<Utc>) -> Self;
    /// Downloaded, unwatched, position > 30s, last played within 7 days of `now`.
    pub fn is_in_progress(&self, now: DateTime<Utc>) -> bool;
    /// Downloaded, unwatched, recorded duration < 900s.
    pub fn is_quick_watch(&self) -> bool;
}
```

```rust
// src/domain/services/video_searcher.rs
impl VideoSearcher {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self;
}

pub trait VideoSearcherApi: Send + Sync {
    // ...list, list_for_channel, list_recent unchanged
    /// Videos in progress across sources, last played first, once per YouTube video.
    fn list_continue_watching(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;
    /// Quick watches across sources, newest sync first, once per YouTube video.
    fn list_quick_watches(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;
}

impl VideoSearcher {
    // Channels are collected before playlists, and the sort is stable, so
    // dedupe keeps the channel copy when there is one (its card has an avatar).
    // Watch state is shared by every copy, so any copy is correct.
    fn downloaded_across_sources(&self) -> Result<Vec<RecentVideo>, ListVideosError>;
    fn once_per_youtube_video(videos: Vec<RecentVideo>) -> Vec<RecentVideo>;
}
```

```rust
// src/application/http/videos/mod.rs
pub async fn list_continue_watching_videos(
    State(video_searcher): State<VideoSearcher>,
    Query(query): Query<ListRecentVideosQuery>,
) -> Result<Json<Vec<RecentVideoResponse>>, ApiError>;

pub async fn list_quick_watch_videos(
    State(video_searcher): State<VideoSearcher>,
    Query(query): Query<ListRecentVideosQuery>,
) -> Result<Json<Vec<RecentVideoResponse>>, ApiError>;

// dto.rs
pub struct RecentVideoResponse {
    // ...existing fields
    pub position_seconds: i64,
}
```

## Call Stack

**Record progress** (changed):
`POST /videos/{id}/progress {position_seconds, duration_seconds?}` → `record_video_progress` → `VideoWatchStateUpdater::update(&youtube_id, position, reported_duration)` → for each copy: `Video::update_watch_state(position, reported_duration, clock.now())` (sets `last_played_at`) → `VideoRepository::update(&video)`

**Continue watching**:
`GET /videos/continue-watching?limit=N` → `list_continue_watching_videos` → limit = `limit.unwrap_or(20).min(100)` → `VideoSearcher::list_continue_watching(limit)` → `downloaded_across_sources()` (channel and playlist repos → `VideoRepository::find`) → filter `video.is_in_progress(clock.now())` → sort by `last_played_at` desc → `once_per_youtube_video` → truncate(limit) → `RecentVideoResponse::from`

**Quick watches**:
`GET /videos/quick-watches?limit=N` → `list_quick_watch_videos` → same limit → `VideoSearcher::list_quick_watches(limit)` → `downloaded_across_sources()` → filter `video.is_quick_watch()` → sort by `created_at` desc → `once_per_youtube_video` → truncate(limit) → `RecentVideoResponse::from`

**Home view**:
`Home` → `usePolling(fetchContinueWatchingVideos)`, `usePolling(fetchRecentVideos)`, `usePolling(fetchQuickWatchVideos)` → `HomeSection` per list (a new section renders nothing when its list is empty) → `VideoGrid` (continue-watching cards render `WatchProgressBar` with `position_seconds / duration_seconds`)

## Test Plan

### 1. Behaviour (acceptance, `src/application/http/videos/mod.rs` and `channels/mod.rs`)

Record progress:
1. `it_should_record_the_last_played_time_on_every_copy`: after a progress report, every copy has `last_played_at = clock now`.
2. `it_should_record_the_last_played_time_even_if_the_watch_state_is_unchanged`: a watched video reported at ≤10% stays watched, and `last_played_at` is set.
3. `it_should_not_change_the_last_played_time_when_marking_a_channel_watched` (in `channels/mod.rs`): the videos' `last_played_at` is unchanged after marking the channel watched.

Continue watching:
4. `it_should_list_no_continue_watching_videos_if_none_in_progress`: empty list.
5. `it_should_list_a_recently_started_video_in_continue_watching`: an unwatched video at 120s, played 2 days ago, is returned with `position_seconds: 120`.
6. `it_should_order_continue_watching_by_last_played_first`
7. `it_should_exclude_videos_last_played_over_a_week_ago_from_continue_watching`: excluded at 7 days + 1s, included at exactly 7 days.
8. `it_should_exclude_barely_started_videos_from_continue_watching`: excluded at 30s, included at 31s.
9. `it_should_exclude_watched_and_never_played_videos_from_continue_watching`
10. `it_should_exclude_not_downloaded_videos_from_continue_watching`
11. `it_should_list_a_continue_watching_video_once_across_sources`: a video tracked by a channel and a playlist is returned once, with the channel source.
12. `it_should_honor_and_cap_the_continue_watching_limit`

Quick watches:
13. `it_should_list_no_quick_watches_if_none_short`: empty list.
14. `it_should_list_short_unwatched_videos_as_quick_watches_newest_first`
15. `it_should_exclude_videos_of_15_minutes_or_more_from_quick_watches`: excluded at 900s, included at 899s.
16. `it_should_exclude_videos_without_duration_from_quick_watches`
17. `it_should_exclude_watched_and_not_downloaded_videos_from_quick_watches`
18. `it_should_list_a_quick_watch_once_across_sources`
19. `it_should_honor_and_cap_the_quick_watches_limit`

### 2. Infrastructure

`SqliteVideoRepository`:
20. `it_should_round_trip_a_video_with_a_last_played_time`
21. `it_should_round_trip_a_video_never_played`

Migrations (`sqlite_migrations.rs`):
22. `it_should_backfill_the_last_played_time_of_part_watched_videos_when_migrating`: an unwatched video with a position gets `last_played_at = updated_at`. Watched videos and videos with position 0 stay NULL.
