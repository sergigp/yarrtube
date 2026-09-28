## Files

- `migrations/0005_last_played_at.sql`: adds `videos.last_played_at` and backfills it with the migration time for unwatched videos with a saved position (no play time was recorded before, and progress never touches `updated_at`).
- `src/infrastructure/shared/sqlite_migrations.rs`: registers migration `0005`.
- `src/domain/video/video.rs`: `last_played_at` field. `update_watch_state` sets it. New rules `is_in_progress` and `is_quick_watch` and their constants live on the entity.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: reads and writes the new column (`VIDEO_COLUMNS`, `save`, `update`, row mapping). New port method `find_many`, loading several videos in one query.
- `src/domain/video/home_videos.rs`: `HomeVideos`, the three home sections' videos.
- `src/domain/video/home_limits.rs`: `HomeLimits`, how many videos each home section holds.
- `src/domain/services/video_searcher.rs`: `list_continue_watching`, `list_quick_watches` and `list_home`, all through one private section-picking helper over a single cross-source collection. Takes a `Clock` for the 7-day window. Reuses `recent_from_playlists` / `recent_from_channels` with `list_recent`, which now load each source's videos with `find_many`.
- `src/application/http/videos/mod.rs`: three handlers. The two section handlers reuse `ListRecentVideosQuery` and the recent limits, and share one private body with `list_recent_videos`; `list_home_videos` holds the home limits (6, 6, 18).
- `src/application/http/videos/dto.rs`: `RecentVideoResponse` gains `position_seconds`. New `HomeResponse`.
- `src/application/http/mod.rs`: routes `/videos/continue-watching`, `/videos/quick-watches` and `/videos/home`.
- `src/serve.rs`: passes the clock to `VideoSearcher::new`.
- `src/application/http/channels/mod.rs`: test only (marking a channel watched keeps last played times).
- `src/application/http/tasks/mod.rs`: test only (a `Video` fixture gains `last_played_at: None`).
- `web/src/api.js`: `fetchHomeVideos`; the per-section home fetchers are removed.
- `web/src/components/Home.jsx`: renders Continue watching, Quick watches and Latest videos, in that order, from one `fetchHomeVideos` poll; the first two are hidden when empty, loading or failing.
- `web/src/components/WatchProgressBar.jsx`: thin bar over a thumbnail showing position / duration.
- `smoke-tests/tests/playlist.spec.js`: pauses the playlist's video (about 10 minutes long) halfway; the home view shows it under "Continue watching" with a progress bar and not under "Latest videos", and the section headings render in order.
- `smoke-tests/tests/channel.spec.js`: its part-playback seeks to half the duration instead of 5s, so the resume check holds whatever the channel's newest video lasts.

## Types & Signatures

```sql
-- migrations/0005_last_played_at.sql
ALTER TABLE videos ADD COLUMN last_played_at TEXT;
UPDATE videos SET last_played_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
 WHERE watched_at IS NULL AND playback_position_seconds > 0;
```

```rust
// src/domain/video/video.rs
const IN_PROGRESS_WINDOW: Duration = Duration::days(7);
const IN_PROGRESS_MIN_POSITION_SECONDS: i64 = 30;
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
    /// Unwatched, position > 30s, last played within 7 days of `now`.
    /// Doesn't check the download status: callers pass downloaded videos.
    pub fn is_in_progress(&self, now: DateTime<Utc>) -> bool;
    /// Unwatched, recorded duration < 900s. Doesn't check the download
    /// status: callers pass downloaded videos.
    pub fn is_quick_watch(&self) -> bool;
}
```

```rust
// src/infrastructure/repositories/sqlite_video_repository.rs
pub trait VideoRepository: Send + Sync {
    // ...existing methods
    /// The stored videos among `ids`, in the order given, skipping ids with
    /// no stored video. One query however many ids.
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<Video>>;
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
        video_metadata_repository: Arc<dyn VideoMetadataRepository>, // added by video-detail-metadata
        clock: Arc<dyn Clock>,
    ) -> Self;
}

pub trait VideoSearcherApi: Send + Sync {
    // ...list, list_for_channel, list_recent unchanged
    /// Videos in progress across sources, last played first, once per YouTube video.
    fn list_continue_watching(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;
    /// Quick watches across sources, newest sync first, once per YouTube video.
    fn list_quick_watches(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;
    /// The three home sections from one collection: continue watching, then
    /// quick watches without those shown above, then latest (once per source)
    /// without those shown in either. Only shown videos are excluded.
    fn list_home(&self, limits: HomeLimits) -> Result<HomeVideos, ListVideosError>;
}

impl VideoSearcher {
    // The videos of `videos` that `keep` accepts, deduplicated, highest
    // `newest` first, truncated to `limit`.
    fn pick_once<K: Ord>(videos: &[RecentVideo], keep: impl Fn(&Video) -> bool, newest: impl Fn(&Video) -> K, limit: usize) -> Vec<RecentVideo>;
    // Every downloaded video of every channel, then of every playlist.
    fn downloaded_across_sources(&self) -> Result<Vec<RecentVideo>, ListVideosError>;
    // Keeps the first copy of each YouTube video. Applied before sorting,
    // while channels still come before playlists, so the channel copy wins
    // (its card has an avatar) whatever each copy's own timestamps. Watch
    // state is shared by every copy, so any copy is correct.
    fn once_per_youtube_video(videos: Vec<RecentVideo>) -> Vec<RecentVideo>;
}
```

```rust
// src/domain/video/home_videos.rs
pub struct HomeVideos {
    pub continue_watching: Vec<RecentVideo>,
    pub quick_watches: Vec<RecentVideo>,
    pub latest: Vec<RecentVideo>,
}

// src/domain/video/home_limits.rs
pub struct HomeLimits {
    pub continue_watching: usize,
    pub quick_watches: usize,
    pub latest: usize,
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

// Shared by list_recent_videos and the two handlers above: limit default 20,
// capped at 100, run_blocking, error mapping, response mapping.
async fn list_across_sources(
    video_searcher: VideoSearcher,
    query: &ListRecentVideosQuery,
    list: fn(&VideoSearcher, usize) -> Result<Vec<RecentVideo>, ListVideosError>,
) -> Result<Json<Vec<RecentVideoResponse>>, ApiError>;

pub async fn list_home_videos(
    State(video_searcher): State<VideoSearcher>,
) -> Result<Json<HomeResponse>, ApiError>;

// dto.rs
pub struct RecentVideoResponse {
    // ...existing fields
    pub position_seconds: i64,
}

pub struct HomeResponse {
    pub continue_watching: Vec<RecentVideoResponse>,
    pub quick_watches: Vec<RecentVideoResponse>,
    pub latest: Vec<RecentVideoResponse>,
}
```

## Call Stack

**Record progress** (changed):
`POST /videos/{id}/progress {position_seconds, duration_seconds?}` → `record_video_progress` → `VideoWatchStateUpdater::update(&youtube_id, position, reported_duration)` → for each copy: `Video::update_watch_state(position, reported_duration, clock.now())` (sets `last_played_at`) → `VideoRepository::update(&video)`

**Continue watching**:
`GET /videos/continue-watching?limit=N` → `list_continue_watching_videos` → `list_across_sources` (limit = `limit.unwrap_or(20).min(100)`) → `VideoSearcher::list_continue_watching(limit)` → `downloaded_across_sources()` (channel then playlist repos → `VideoRepository::find_many` per source) → `pick_once`: filter `video.is_in_progress(clock.now())` → `once_per_youtube_video` → sort by `last_played_at` desc → truncate(limit) → `RecentVideoResponse::from`

**Quick watches**:
`GET /videos/quick-watches?limit=N` → `list_quick_watch_videos` → `list_across_sources` → `VideoSearcher::list_quick_watches(limit)` → `downloaded_across_sources()` → `pick_once`: filter `video.is_quick_watch()` → `once_per_youtube_video` → sort by `created_at` desc → truncate(limit) → `RecentVideoResponse::from`

**Home**:
`GET /videos/home` → `list_home_videos` → `VideoSearcher::list_home(HomeLimits { continue_watching: 6, quick_watches: 6, latest: 18 })` → `downloaded_across_sources()` once → continue watching = `pick_once(is_in_progress(now), last_played_at, 6)` → quick watches = `pick_once(is_quick_watch && not shown above, created_at, 6)` → latest = videos not shown above, sorted by `created_at` desc, truncate(18) → `HomeResponse` (each list via `RecentVideoResponse::from`)

**Home view**:
`Home` → `usePolling(fetchHomeVideos)` (→ `GET /videos/home`) → `HomeSection` per list, in that order (a new section renders nothing while loading, on error or when its list is empty) → `VideoGrid` (continue-watching cards render `WatchProgressBar` with `position_seconds / duration_seconds`)

## Test Plan

### 1. Behaviour (acceptance, `src/application/http/videos/mod.rs` and `channels/mod.rs`)

Record progress:
1. `it_should_record_the_last_played_time_on_every_copy`: after a progress report, every copy has `last_played_at = clock now`.
2. `it_should_record_the_last_played_time_even_if_the_watch_state_is_unchanged`: a watched video reported at exactly 10% stays watched, and `last_played_at` is set.
3. Existing `it_should_mark_the_video_watched_at_90_percent`, `it_should_mark_a_watched_video_unwatched_past_10_percent_of_a_rewatch` and `it_should_use_the_reported_duration_if_none_is_recorded` also expect `last_played_at`, so every branch of `update_watch_state` sets it. `it_should_record_progress_on_every_copy_of_the_video` and `it_should_keep_a_watched_video_watched_early_in_a_rewatch` are removed as duplicates of 1 and 2.
4. `it_should_not_change_the_last_played_time_when_marking_a_channel_watched` (in `channels/mod.rs`): the videos' `last_played_at` is unchanged after marking the channel watched.

Continue watching:
5. `it_should_list_no_continue_watching_videos_if_none_in_progress`: empty list.
6. `it_should_list_a_recently_started_video_in_continue_watching`: an unwatched video at 120s, played 2 days ago, is returned with `position_seconds: 120`.
7. `it_should_order_continue_watching_by_last_played_first`
8. `it_should_exclude_videos_last_played_over_a_week_ago_from_continue_watching`: excluded at 7 days + 1s, included at exactly 7 days.
9. `it_should_exclude_barely_started_videos_from_continue_watching`: excluded at 30s, included at 31s.
10. `it_should_exclude_watched_and_never_played_videos_from_continue_watching`
11. `it_should_exclude_not_downloaded_videos_from_continue_watching`
12. `it_should_list_a_continue_watching_video_once_across_sources`: a video tracked by a channel and a playlist is returned once, with the channel source, even when the playlist copy was played later.
13. `it_should_honor_and_cap_the_continue_watching_limit`

Quick watches:
14. `it_should_list_no_quick_watches_if_none_short`: empty list.
15. `it_should_list_short_unwatched_videos_as_quick_watches_newest_first`
16. `it_should_exclude_videos_of_15_minutes_or_more_from_quick_watches`: excluded at 900s, included at 899s.
17. `it_should_exclude_videos_without_duration_from_quick_watches`
18. `it_should_exclude_watched_and_not_downloaded_videos_from_quick_watches`
19. `it_should_list_a_quick_watch_once_across_sources`: returned once, with the channel source, even when the playlist copy is newer.
20. `it_should_honor_and_cap_the_quick_watches_limit`

Home (composition only; the section rules are covered through the continue watching and quick watches endpoints above):
21. `it_should_list_a_downloaded_video_under_latest_on_home`: a never-played, long video is only under `latest`.
22. `it_should_list_a_started_video_under_continue_watching_only_on_home`: not repeated under `latest`.
23. `it_should_list_a_short_video_under_quick_watches_only_on_home`: not repeated under `latest`.
24. `it_should_not_repeat_a_continue_watching_video_in_quick_watches_on_home`: a started short video is only under `continue_watching`.
25. `it_should_show_videos_left_out_of_a_full_section_further_down_on_home`: 7 started and 7 short videos: 6 of each shown in their sections, the 7th started one and the 7th short one under `latest`.
26. `it_should_cap_latest_on_home_at_18`: 20 plain videos, the newest 18 under `latest`.

### 2. Infrastructure

`SqliteVideoRepository`:
27. `it_should_round_trip_a_video_with_a_last_played_time`
28. `it_should_round_trip_a_video_never_played`
29. `it_should_find_many_videos_in_the_order_given_skipping_missing_ones`
30. `it_should_find_many_of_no_ids`

Migrations (`sqlite_migrations.rs`):
31. `it_should_backfill_the_last_played_time_of_part_watched_videos_when_migrating`: an unwatched video with a position gets `last_played_at` within the migration window. Watched videos and videos with position 0 stay NULL.
