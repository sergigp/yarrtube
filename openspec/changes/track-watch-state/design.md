## Files

**Backend**
- `migrations/0002_watch_state.sql`: adds `watched_at TEXT NULL` and `playback_position_seconds INTEGER NOT NULL DEFAULT 0` to `videos`, plus an index on `videos(youtube_id)`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers `0002` after the baseline.
- `src/domain/video/playback_position.rs`: new `PlaybackPosition` value object (non-negative whole seconds).
- `src/domain/video/video.rs`: `watched_at`/`playback_position` fields, plus the `record_progress` and `mark_watched` transitions.
- `src/domain/video/errors.rs`: new `UpdateWatchStateError`.
- `src/domain/video/mod.rs`: exports `PlaybackPosition` and `UpdateWatchStateError`.
- `src/domain/channel/channel_view.rs`: new flat `ChannelView` (only the channel fields the listing exposes, plus the unwatched count).
- `src/domain/channel/mod.rs`: exports `ChannelView`.
- `src/domain/services/video_watch_state_updater.rs`: new `VideoWatchStateUpdater` service holding every watch-state use case.
- `src/domain/services/channel_searcher.rs`: `search_all` returns `Vec<ChannelView>` and gains the channel-video and video repositories.
- `src/domain/services/mod.rs`: exports `VideoWatchStateUpdater` and `VideoWatchStateUpdaterApi`.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: new columns in `save`/`find`/`update`/`row_to_video`, and a new `find_by_youtube_id` read.
- `src/application/http/videos/mod.rs`: `record_video_progress` handler.
- `src/application/http/videos/dto.rs`: `RecordProgressRequest`, plus `watched`/`position_seconds` on `VideoResponse` and `watched` on `RecentVideoResponse`.
- `src/application/http/channels/mod.rs`: `mark_channel_watched` handler; `list_channels` maps `ChannelView`.
- `src/application/http/channels/dto.rs`: new flat `ChannelListItemResponse` (only the fields the SPA reads, plus `unwatched_count`). `create_channel` keeps returning `ChannelResponse`.
- `src/application/http/validation.rs`: `MISSING_POSITION` message.
- `src/application/http/mod.rs`: `ApiServices.video_watch_state_updater` and the two new routes.
- `src/serve.rs`: constructs `VideoWatchStateUpdater` and passes the extra repositories to `ChannelSearcher`.

**SPA**
- `web/src/api.js`: `recordVideoProgress`, `beaconVideoProgress`, `markChannelWatched`.
- `web/src/useWatchProgress.js`: new hook that attaches to a `<video>`, resumes from the saved position and reports progress.
- `web/src/components/WatchedTick.jsx`: new tick overlay for a thumbnail.
- `web/src/components/ChannelDetail.jsx`: uses the hook, shows the tick and the "Mark all watched" control. The selection is kept by id so resume reads fresh polled data.
- `web/src/components/PlaylistDetail.jsx`: uses the hook and shows the tick.
- `web/src/components/Home.jsx`: shows the tick on the recent-videos grid.
- `web/src/components/Sidebar.jsx`: unwatched badge and mark-watched row action on channel rows only.

**Smoke tests**
- `smoke-tests/helpers/sidebar.js`: `markItemWatched`.
- `smoke-tests/tests/channel.spec.js`: covers the progress and mark channel watched routes through the UI.

## Types & Signatures

```sql
-- migrations/0002_watch_state.sql
ALTER TABLE videos ADD COLUMN watched_at TEXT;
ALTER TABLE videos ADD COLUMN playback_position_seconds INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_videos_youtube_id ON videos (youtube_id);
```

```rust
// domain/video/playback_position.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlaybackPosition(i64);

impl PlaybackPosition {
    pub fn new(seconds: i64) -> Result<Self, ValidationError>; // "Playback position must not be negative (got N)"
    pub fn start() -> Self;                                     // 0
    pub fn seconds(&self) -> i64;
}
```

```rust
// domain/video/video.rs
pub struct Video {
    // ...existing fields...
    pub watched_at: Option<DateTime<Utc>>,
    pub playback_position: PlaybackPosition,
}

const WATCHED_THRESHOLD: f64 = 0.9;
const REWATCH_RESET_THRESHOLD: f64 = 0.1;

impl Video {
    // Video::create -> watched_at: None, playback_position: PlaybackPosition::start()
    // reset_for_redownload keeps watch state (struct update, untouched).

    /// Duration is `self.duration_seconds`, else `reported_duration_seconds`; non-positive = unknown.
    /// unwatched: pos >= 90% -> mark_watched; else position = pos (also when duration unknown).
    /// watched:   10% < pos < 90% -> watched_at = None, position = pos; else unchanged.
    pub fn record_progress(
        self,
        position: PlaybackPosition,
        reported_duration_seconds: Option<i64>,
        now: DateTime<Utc>,
    ) -> Self;
    pub fn mark_watched(self, now: DateTime<Utc>) -> Self; // watched_at = now, position = start
    pub fn is_watched(&self) -> bool;
}
```

```rust
// domain/video/errors.rs
#[derive(Debug)]
pub enum UpdateWatchStateError {
    VideoNotFound(VideoId),        // "video {id} not found"
    ChannelNotFound(ChannelHandle), // "channel {id} not found"
    Repository(anyhow::Error),
}
```

```rust
// domain/channel/channel_view.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelView {
    pub id: ChannelHandle,
    pub name: String,
    pub path: PlaylistPath,
    pub avatar_filename: Option<String>,
    pub unwatched_count: usize,
}
```

```rust
// infrastructure/repositories/sqlite_video_repository.rs
pub trait VideoRepository: Send + Sync {
    // ...existing...
    /// Every stored copy of a YouTube video, across all playlists/channels.
    fn find_by_youtube_id(&self, youtube_id: &VideoId) -> anyhow::Result<Vec<Video>>;
}
```

```rust
// domain/services/video_watch_state_updater.rs
#[derive(Clone)]
pub struct VideoWatchStateUpdater {
    video_repository: Arc<dyn VideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoWatchStateUpdater {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self;
}

pub trait VideoWatchStateUpdaterApi: Send + Sync {
    /// Applies `Video::record_progress` to every stored copy of `youtube_id`.
    fn record_progress(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration_seconds: Option<i64>,
    ) -> Result<(), UpdateWatchStateError>;
    /// Marks every `Downloaded` video of the channel watched, with every
    /// stored copy of each; pending/in-flight videos are left unchanged.
    fn mark_channel_watched(&self, channel_id: &ChannelHandle) -> Result<(), UpdateWatchStateError>;
}
```

```rust
// domain/services/channel_searcher.rs
impl ChannelSearcher {
    pub fn new(
        repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
    ) -> Self;
}

pub trait ChannelSearcherApi: Send + Sync {
    /// Every channel with its count of `Downloaded`, unwatched videos.
    fn search_all(&self) -> anyhow::Result<Vec<ChannelView>>;
}
```

```rust
// application/http/videos/dto.rs
#[derive(Debug, Deserialize)]
pub struct RecordProgressRequest {
    #[serde(default)]
    pub position_seconds: Option<i64>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
}

pub struct VideoResponse {
    // ...existing...
    pub watched: bool,
    pub position_seconds: i64,
}

pub struct RecentVideoResponse {
    // ...existing...
    pub watched: bool,
}
```

```rust
// application/http/channels/dto.rs
#[derive(Debug, Serialize, PartialEq)]
pub struct ChannelListItemResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub avatar_filename: Option<String>,
    pub unwatched_count: usize,
}
impl From<ChannelView> for ChannelListItemResponse;
```

```rust
// application/http/videos/mod.rs
pub async fn record_video_progress(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(youtube_id): Path<String>,
    Json(request): Json<RecordProgressRequest>,
) -> Result<StatusCode, ApiError>; // 204

// application/http/channels/mod.rs
pub async fn mark_channel_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(handle): Path<String>,
) -> Result<StatusCode, ApiError>; // 204
pub async fn list_channels(
    State(channel_searcher): State<ChannelSearcher>,
) -> Result<Json<Vec<ChannelListItemResponse>>, ApiError>;

// UpdateWatchStateError mapping (one fn, shared): VideoNotFound | ChannelNotFound -> 400, Repository -> 500
```

```
// routes (application/http/mod.rs)
POST   /videos/{id}/progress   -> videos::record_video_progress
POST   /channels/{handle}/watched -> channels::mark_channel_watched
```

```js
// web/src/api.js
export async function recordVideoProgress(youtubeId, { position_seconds, duration_seconds })
export function beaconVideoProgress(youtubeId, { position_seconds, duration_seconds }) // navigator.sendBeacon, Blob type application/json
export async function markChannelWatched(handle)

// web/src/useWatchProgress.js
// Resumes on loadedmetadata when !video.watched && video.position_seconds > 0.
// Reports on timeupdate (throttled to 15s), pause, ended, and video change/unmount;
// uses beaconVideoProgress on pagehide.
export function useWatchProgress(videoRef, video)

// web/src/components/WatchedTick.jsx
export function WatchedTick({ watched })
```

## Call Stack

Every write in `VideoWatchStateUpdater` loads whole entities, applies a transition and saves the whole entity with the existing `update`. There are no partial-column writes. Badges and ticks refresh on the SPA's existing 3s poll, so the SPA needs no response body and no manual refetch.

Record progress:
```
POST /api/videos/{id}/progress {position_seconds, duration_seconds?}
  record_video_progress(State<VideoWatchStateUpdater>, Path(id), Json(request))
    VideoId::new(id)?
    PlaybackPosition::new(required(request.position_seconds, MISSING_POSITION)?)?
    run_blocking(video_watch_state_updater.record_progress(&youtube_id, position, request.duration_seconds))
      copies = find_copies(youtube_id)                  -> video_repository.find_by_youtube_id; empty -> VideoNotFound
      now = clock.now()
      copies.map(|v| v.record_progress(position, reported_duration_seconds, now))
            .try_for_each(|v| video_repository.update(&v))
    -> 204
```

Mark channel watched:
```
POST /api/channels/{handle}/watched
  mark_channel_watched(State<VideoWatchStateUpdater>, Path(handle))
    ChannelHandle::new(handle)?
    run_blocking(video_watch_state_updater.mark_channel_watched(&channel_id))
      channel_repository.find(channel_id)               -> None -> ChannelNotFound
      channel_video_repository.list_for_channel(channel_id)
        .map(|cv| video_repository.find(&cv.video_id))
        .filter(status == Downloaded && !is_watched)
        -> youtube_ids (deduplicated)
      youtube_ids.try_for_each(|id| self.mark_copies_watched(id))   (private helper)
        mark_copies_watched: video_repository.find_by_youtube_id(id)
                             .map(|v| v.mark_watched(now)).try_for_each(update)
    -> 204
```

List channels with unwatched count:
```
GET /api/channels
  list_channels(State<ChannelSearcher>)
    run_blocking(channel_searcher.search_all())
      channel_repository.list()
        .map(|channel| ChannelView { id, name, path, avatar_filename (from channel), unwatched_count: count_unwatched(&channel.id) })
          count_unwatched: channel_video_repository.list_for_channel
                           .filter_map(video_repository.find)
                           .filter(status == Downloaded && !is_watched).count()
    -> Json(Vec<ChannelListItemResponse>)
```

SPA playback:
```
ChannelDetail / PlaylistDetail
  selectedVideo = videos.find(id == selectedId)       (fresh from poll)
  useWatchProgress(videoRef, selectedVideo)
    loadedmetadata -> if !watched && position_seconds > 0: el.currentTime = position_seconds
    timeupdate     -> if now - lastReport >= 15s: recordVideoProgress(id, {floor(currentTime), floor(duration)})
    pause | ended  -> recordVideoProgress(...)
    cleanup (video change / unmount) -> recordVideoProgress(...)
    pagehide       -> beaconVideoProgress(...)
Sidebar channel row
  badge = item.unwatched_count > 0
  mark-watched control -> markChannelWatched(item.id)
```

## Test Plan

**1. Behaviour tests** (acceptance, entering through the HTTP handlers, real SQLite and `FixedClock`)

`application/http/videos/mod.rs`, record progress:
1. `it_should_record_playback_progress`: an unwatched video with a 100s duration, position 30. Returns 204, and the stored video has position 30 and is unwatched.
2. `it_should_mark_the_video_watched_at_90_percent`: position 90 of 100. Stored video has `watched_at = now` and position 0.
3. `it_should_use_the_reported_duration_if_none_is_recorded`: no recorded duration, `duration_seconds: 100` in the request, position 95. Video becomes watched.
4. `it_should_only_record_the_position_if_duration_is_unknown`: no duration anywhere, position 500. Position 500, still unwatched.
5. `it_should_keep_a_watched_video_watched_early_in_a_rewatch`: watched video, position 10 of 100. Stored video unchanged.
6. `it_should_mark_a_watched_video_unwatched_past_10_percent_of_a_rewatch`: watched video, position 11 of 100. `watched_at = None`, position 11.
7. `it_should_keep_a_watched_video_watched_when_playing_on_past_90_percent`: watched video, position 95 of 100. Stored video unchanged.
8. `it_should_record_progress_on_every_copy_of_the_video`: same YouTube ID stored in a channel and a playlist. Both copies are updated identically.
9. `it_should_fail_to_record_progress_of_an_unknown_video`: `Err(ApiError::bad_request("video x not found"))`, stored videos unchanged.
10. `it_should_fail_to_record_progress_if_position_missing`: exact missing-position 400, on an unmigrated connection.
11. `it_should_fail_to_record_progress_if_invalid_position_provided`: position -1. Exact `PlaybackPosition` 400 message.

`application/http/channels/mod.rs`:

12. `it_should_mark_every_downloaded_channel_video_watched`: the channel has a downloaded video, a pending video, and a downloaded video also stored in a playlist. Both downloaded videos and the playlist copy become watched; the pending video is unchanged.
13. `it_should_fail_to_mark_watched_a_missing_channel`: 400 "channel @missing not found", videos unchanged.
14. `it_should_fail_to_mark_watched_if_invalid_handle_provided`: exact `ChannelHandle` 400 message.
15. `it_should_count_unwatched_downloaded_videos_when_listing_channels`: one channel with downloaded-unwatched ×2, downloaded-watched ×1 and pending ×1. Whole flat `ChannelListItemResponse` with `unwatched_count: 2`. (The existing `it_should_list_all_channels`/`it_should_list_no_channels` are updated to the new response and constructor.)

`application/http/videos/mod.rs`, listings:

16. `it_should_include_watch_state_when_listing_channel_videos`: one watched video and one video at position 42. Whole `VideoResponse`s with `watched`/`position_seconds`.
17. `it_should_include_watch_state_when_listing_playlist_videos`: same, through the playlist listing.
18. `it_should_include_whether_recent_videos_were_watched`: a watched downloaded video appears with `watched: true`.

`application/tasks/reconcile_channel_task.rs`:

19. `it_should_keep_watch_state_when_redownloading_a_missing_file`: a watched `Downloaded` video whose file is missing. After the reconcile the video is `Pending` and still watched.

`domain/video/playback_position.rs` (value object):

20. `it_should_accept_zero_and_positive_positions`: `Ok(PlaybackPosition(0))` and `Ok(PlaybackPosition(120))`.
21. `it_should_reject_a_negative_position`: `Err(ValidationError("Playback position must not be negative (got -1)"))`.

**2. Infrastructure tests**

`infrastructure/shared/sqlite_migrations.rs`:

22. `it_should_default_existing_videos_to_unwatched_when_migrating`: apply only the baseline, insert a `videos` row with raw SQL, then `apply`. The row reads back with `watched_at` NULL and position 0.

`infrastructure/repositories/sqlite_video_repository.rs`:

23. `it_should_round_trip_a_watched_video_with_a_playback_position`: save, then `find` returns the whole video unchanged.
24. `it_should_find_every_copy_of_a_youtube_video`: three rows, two sharing a YouTube ID. `find_by_youtube_id` returns exactly those two.
25. `it_should_find_no_copies_of_an_unknown_youtube_video`: `Ok(vec![])`.
