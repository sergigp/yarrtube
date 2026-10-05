## Files

Backend:
- `src/domain/video/video.rs`: `update_watch_state` takes `was_watched` and ignores stale reports
- `src/domain/video/errors.rs`: `UpdateWatchStateError::VideoNotDownloaded`
- `src/domain/services/video_watch_state_updater.rs`: `update` forwards `was_watched`; new `mark_video_watched`
- `src/application/http/videos/dto.rs`: `RecordProgressRequest.was_watched`
- `src/application/http/videos/mod.rs`: new `mark_video_watched` handler; `record_video_progress` requires `was_watched`; error mapping
- `src/application/http/validation.rs`: `MISSING_WAS_WATCHED` message
- `src/application/http/mod.rs`: route `POST /videos/{id}/watched`

Web:
- `web/src/api/types.ts`: `VideoProgress.was_watched`
- `web/src/api/client.ts`: `markVideoWatched`
- `web/src/api/queries.ts`: `useMarkVideoWatched` (mark, then refetch library and home)
- `web/src/hooks/useWatchProgress.ts`: playback session; new session on a watched flip; reports `was_watched`
- `web/src/components/VideoActionsMenu.tsx` (new): vertical ⋮ with "Mark as watched"
- `web/src/components/Home.tsx`: menu right of the card title block
- `web/src/components/VideoListPane.tsx`: row split into a select `<button>` plus a sibling menu; the selected row renders an empty placeholder of the same size instead of the menu
- `web/src/components/VideoDetail.tsx`: menu in the title row
- `web/src/test/helpers.tsx`: route helpers for the new endpoint if needed

## Types & Signatures

```rust
// src/domain/video/video.rs
impl Video {
    /// A report whose session began with the video unwatched (`was_watched == false`)
    /// on a video that is now watched is stale: returned unchanged, not even
    /// `last_played_at` moves. Otherwise as today.
    pub fn update_watch_state(
        self,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        was_watched: bool,
        now: DateTime<Utc>,
    ) -> Self;
}

// src/domain/video/errors.rs
pub enum UpdateWatchStateError {
    VideoNotFound(VideoId),
    VideoNotDownloaded(VideoId), // new
    ChannelNotFound(ChannelHandle),
    Repository(anyhow::Error),
}

// src/domain/services/video_watch_state_updater.rs
pub trait VideoWatchStateUpdaterApi: Send + Sync {
    fn update(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
        was_watched: bool,
    ) -> Result<bool, UpdateWatchStateError>;
    /// Marks every unwatched copy of `youtube_id` watched, if any copy is `Downloaded`.
    fn mark_video_watched(&self, youtube_id: &VideoId) -> Result<(), UpdateWatchStateError>;
    fn mark_channel_watched(&self, channel_id: &ChannelHandle)
    -> Result<(), UpdateWatchStateError>;
}

// src/application/http/videos/dto.rs
pub struct RecordProgressRequest {
    #[serde(default)] pub position_seconds: Option<i64>,
    #[serde(default)] pub duration_seconds: Option<i64>,
    #[serde(default)] pub was_watched: Option<bool>,
}

// src/application/http/videos/mod.rs
pub async fn mark_video_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(youtube_id): Path<String>,
) -> Result<StatusCode, ApiError>;
// update_watch_state_error: VideoNotDownloaded -> 400
```

```ts
// web/src/api/types.ts
export interface VideoProgress {
  position_seconds: number
  duration_seconds?: number
  was_watched: boolean
}

// web/src/api/client.ts
export function markVideoWatched(youtubeId: string): Promise<void>

// web/src/api/queries.ts
/** Marks the video watched, then refetches channels, playlists (and their video lists) and home. */
export function useMarkVideoWatched(): (youtubeId: string) => Promise<void>

// web/src/components/VideoActionsMenu.tsx
interface VideoActionsMenuProps {
  videoId: string
  title: string          // names the video in the aria-label and the failure alert
  markable: boolean      // false -> "Mark as watched" disabled
  className?: string
}
export function VideoActionsMenu(props: VideoActionsMenuProps): JSX.Element

// web/src/hooks/useWatchProgress.ts: signature unchanged
export function useWatchProgress(videoElement: HTMLVideoElement | null, video: Video | null): void
```

`markable`:
- Home: `!video.watched` (home only lists downloaded videos)
- List rows and detail pane: `!video.watched && video.status === 'DOWNLOADED'`

## Call Stack

Mark a single video watched:
```
VideoActionsMenu "Mark as watched" onSelect
  -> useMarkVideoWatched()(videoId)
     -> markVideoWatched(videoId)                POST /api/videos/{id}/watched
        -> videos::mark_video_watched(State, Path(id))
           -> VideoId::new(id)?
           -> run_blocking(updater.mark_video_watched(&youtube_id))
              -> find_copies(youtube_id)?               (VideoNotFound if none)
              -> any copy Downloaded? else VideoNotDownloaded
              -> copies.filter(!is_watched).map(mark_watched(clock.now()))
              -> video_repository.update(&copy) each
           <- 204
     -> invalidate ['channels'], ['playlists'], ['videos','recent']
  (failure) -> window.alert("Failed to mark \"<title>\" watched: <message>")
```

Record progress (stale guard):
```
useWatchProgress report()
  -> recordVideoProgress(id, { position_seconds, duration_seconds, was_watched: sessionWatched })
     | beaconVideoProgress(...)                       same body
     -> videos::record_video_progress(State, Path(id), Json(req))
        -> VideoId::new, PlaybackPosition::new(required(position)), VideoDuration::new?,
           required(req.was_watched, MISSING_WAS_WATCHED)?
        -> updater.update(&id, position, duration, was_watched)
           -> copies.map(|v| v.update_watch_state(position, duration, was_watched, now))
              stale (!was_watched && v.is_watched()) -> v unchanged
           -> repository.update each; returns any-watched
        <- 200 { watched }
```

Playback session (web):
```
effect [videoElement, videoId, playable]          (attach)
  -> session = { watched: video.watched }; progress = null
  -> report() sends was_watched = session.watched
  -> cleanup: report()

effect [videoId, video.watched]                   (watched flip while attached)
  if video.watched !== session.watched:
    -> session.watched = video.watched; lastWatched = video.watched
    -> element.pause()        (the pause listener is skipped for this pause, so it doesn't report)
    -> element.currentTime = 0
    -> progress = null; lastReportedPosition = 0
```
The two effects share state through a ref (`sessionRef`), so the flip resets the session without re-running the attach effect. If it re-ran, its cleanup would report the stale position.

## Test Plan

### 1. Behaviour tests

Rust: `src/application/http/videos/mod.rs` (handler + real SQLite repos + `FixedClock`)
1. `it_should_mark_a_video_watched`: 204; the copy is watched, position 0, `watched_at` = clock
2. `it_should_mark_every_copy_of_a_video_watched`: channel and playlist copies are both watched
3. `it_should_not_change_the_last_played_time_when_marking_a_video_watched`: `last_played_at` unchanged
4. `it_should_leave_an_already_watched_video_unchanged_when_marking_it_watched`: 204; `watched_at` keeps the earlier time
5. `it_should_fail_to_mark_watched_a_video_not_downloaded`: 400; copy unchanged
6. `it_should_fail_to_mark_watched_an_unknown_video`: 400
7. `it_should_fail_to_mark_watched_if_invalid_video_id_provided`: 400
8. `it_should_ignore_a_stale_progress_report_on_a_video_marked_watched`: `was_watched: false`, 40% on a watched video → 200 `watched: true`; video unchanged, incl. `last_played_at`
9. `it_should_fail_to_record_progress_if_was_watched_missing`: 400; nothing recorded
(Existing progress tests gain `was_watched` matching the video's state; the rewatch tests use `true`.)

Web (Vitest):

`VideoActionsMenu.test.tsx`:

10. `marks the video watched when "Mark as watched" is chosen`: POST routed; channels and home refetched
11. `disables "Mark as watched" when not markable`
12. `alerts and leaves the video as it was when marking fails`

`Home.test.tsx`:

13. `marking a continue-watching video watched removes it from the section`

`VideoListPane` (through `ChannelDetail.test.tsx`):

14. `opening a row's menu does not change the selected video`
15. `marking a row watched shows its tick`
20. `the selected row shows no menu`

`VideoDetail.test.tsx`:

16. `marks the selected video watched from the detail pane`

`useWatchProgress.test.tsx`:

17. `reports was_watched as the session's state`
18. `pauses, rewinds and reports nothing when the video becomes watched while loaded`: unmounting after the flip sends no report, so the old position is never sent with `was_watched: true`
19. `reports was_watched true after a flip once playback moves on`

### 2. Infrastructure tests

None. No adapter changes: `VideoRepository` already supports `find_by_youtube_id` and `update`.
