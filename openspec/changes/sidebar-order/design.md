## Files

**Backend**
- `src/infrastructure/repositories/sqlite_channel_repository.rs`: `list` orders by `name COLLATE NOCASE, rowid`.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs`: `list` orders by `name COLLATE NOCASE, rowid`.
- `src/infrastructure/repositories/sqlite_channel_video_repository.rs`: new `list`, every stored `ChannelVideo` across all channels in one query.
- `src/domain/services/channel_view_searcher.rs`: counts in Rust from 3 bulk reads (channels, all channel videos, `VideoRepository::find_many`) instead of one lookup per video. The downloaded-and-unwatched rule stays in the domain.
- `src/domain/video/video.rs`: `is_downloaded_and_unwatched`, the single definition of what a badge counts.
- `src/domain/services/video_watch_state_updater.rs`: uses `is_downloaded_and_unwatched`; `update` returns the resulting watched state.
- `src/application/http/channels/mod.rs`, `src/application/http/playlists/mod.rs`: new ordering tests.
- `src/application/http/videos/mod.rs`, `src/application/http/videos/dto.rs`: the progress handler responds `200 RecordProgressResponse { watched }` instead of `204`.

**Frontend**
- `web/package.json`, `web/package-lock.json`: add `@tanstack/react-query`.
- `web/src/main.jsx`: create the `QueryClient` (`retry: false`, the next poll retries) and wrap `App` in `QueryClientProvider`.
- `web/src/queries.js` (new): query keys, one hook per fetched resource with its refresh interval, `useInvalidateLibrary`, `useLibraryAction` and `useRemoveQuery`.
- `web/src/api.js`: `recordVideoProgress` returns the parsed `{ watched }` body.
- `web/src/sidebarSections.js` (new): pure functions that order, collapse and filter sidebar rows, plus the thresholds (10 / 5 / 15).
- `web/src/usePolling.js`: deleted, replaced by `queries.js`.
- `web/src/components/Sidebar.jsx`: search field, collapse controls, remembered expanded state; uses `useChannels`/`usePlaylists` and wraps sync, mark watched and delete in `useLibraryAction`; delete drops the entry's video-list query.
- `web/src/components/ChannelDetail.jsx`, `PlaylistDetail.jsx`: share the list queries; wrap sync, mark watched and delete in `useLibraryAction`; delete drops the entry's video-list query.
- `web/src/components/Home.jsx`, `TasksView.jsx`: `useRecentVideos` / `useTasks` instead of `usePolling`.
- `web/src/components/AddDialog.jsx`: invalidate the lists after a successful add.
- `web/src/useWatchProgress.js`: invalidate the channel list (exact) only when a non-beacon report returns a `watched` different from the last known state.

**Smoke tests**
- `smoke-tests/tests/channel.spec.js`, `playlist.spec.js`: expect `200` from the progress report.

## Types & Signatures

```rust
// src/infrastructure/repositories/sqlite_channel_video_repository.rs
pub trait ChannelVideoRepository: Send + Sync {
    // ...existing methods unchanged...
    /// Every stored channel video, across all channels.
    fn list(&self) -> anyhow::Result<Vec<ChannelVideo>>;
}
```

```rust
// src/domain/services/channel_view_searcher.rs
// Struct, `new` and ChannelViewSearcherApi::search_all unchanged.
impl ChannelViewSearcher {
    /// Downloaded, unwatched videos per channel. Channels with none are absent.
    fn unwatched_counts(&self) -> anyhow::Result<HashMap<ChannelHandle, usize>>;
    /// `find_many` over the channel videos, kept if downloaded and unwatched.
    fn unwatched_video_ids(&self, channel_videos: &[ChannelVideo]) -> anyhow::Result<HashSet<VideoRecordId>>;
    fn channel_view(channel: Channel, counts: &HashMap<ChannelHandle, usize>) -> ChannelView;
}
// `count_unwatched` (per-video `find`) is removed.
```

```rust
// src/domain/video/video.rs
impl Video {
    pub fn is_downloaded_and_unwatched(&self) -> bool;
}

// src/domain/services/video_watch_state_updater.rs
pub trait VideoWatchStateUpdaterApi: Send + Sync {
    /// Returns whether the video is watched once the report is applied.
    fn update(
        &self,
        youtube_id: &VideoId,
        position: PlaybackPosition,
        reported_duration: Option<VideoDuration>,
    ) -> Result<bool, UpdateWatchStateError>;
    // mark_channel_watched unchanged
}

// src/application/http/videos/dto.rs
#[derive(Serialize)]
pub struct RecordProgressResponse {
    pub watched: bool,
}

// src/application/http/videos/mod.rs
pub async fn record_video_progress(..) -> Result<Json<RecordProgressResponse>, ApiError>;
```

```js
// web/src/queries.js
export const queryKeys = {
  channels: ['channels'],
  playlists: ['playlists'],
  channelVideos: (handle) => ['channels', handle, 'videos'],
  playlistVideos: (id) => ['playlists', id, 'videos'],
  recentVideos: ['videos', 'recent'],
  tasks: ['tasks'],
}
export function useChannels()                // refetchInterval 60_000
export function usePlaylists()               // refetchInterval 60_000
export function useChannelVideos(handle)     // refetchInterval 3_000
export function usePlaylistVideos(id)        // refetchInterval 3_000
export function useRecentVideos()            // refetchInterval 3_000
export function useTasks()                   // refetchInterval 3_000
export function useInvalidateLibrary()       // () => invalidates ['channels'] and ['playlists'] (prefix match)
export function useLibraryAction()           // (action) => async (...args) => { await action(...args); invalidateLibrary() }
export function useRemoveQuery()             // (queryKey) => removeQueries({ queryKey, exact: true })
```

```js
// web/src/api.js
export async function recordVideoProgress(youtubeId, progress) // -> { watched }
```

```js
// web/src/sidebarSections.js
export const UNREAD_CAP = 10
export const CAUGHT_UP_PREVIEW = 5
export const SEARCH_THRESHOLD = 15

// Unread by unwatched_count desc; stable, so ties and caught-up keep the API's name order.
export function orderChannels(channels)                          // -> channels
// First `leadCount` rows, plus the active row if it falls outside them.
export function collapse(rows, leadCount, activeId)              // -> { shown, hiddenCount }
export function channelLeadCount(orderedChannels)                // -> min(unread, UNREAD_CAP) || CAUGHT_UP_PREVIEW
export function matchesSearch(item, text)                        // -> case-insensitive name contains; `text` already trimmed
```

## Call Stack

**List channels**: `GET /api/channels`
1. `list(State<ChannelViewSearcher>)`
2. → `ChannelViewSearcher::search_all()`
   - → `ChannelRepository::list()` (sorted by name, NOCASE)
   - → `unwatched_counts()`
     - → `ChannelVideoRepository::list()` (all channel videos)
     - → `VideoRepository::find_many(&video_ids)` (one `IN (...)` query)
     - → keep videos with `is_downloaded_and_unwatched()`, then fold the channel videos into `HashMap<ChannelHandle, usize>`
   - → each `Channel` → `Self::channel_view(channel, &counts)` with `unwatched_count: counts.get(&id).unwrap_or(0)`
3. → `ChannelListItemResponse::from` for each

**List playlists**: `GET /api/playlists` → unchanged path → `PlaylistRepository::list()` (sorted by name, NOCASE).

**Render sidebar**
1. `Sidebar` → `useChannels()`, `usePlaylists()`, search text state, expanded state per section (localStorage `yarrtube.sidebar.expanded.<section>`, read/write in try/catch)
2. Channels: `orderChannels(channels)` → `sectionView`: searching ? `filter(matchesSearch)` : expanded ? all : `collapse(ordered, channelLeadCount(ordered), activeChannelId).shown`
3. Playlists: `sectionView`: searching ? `filter(matchesSearch)` : expanded ? all : `collapse(playlists, CAUGHT_UP_PREVIEW, activePlaylistId).shown`

`sectionView` always runs `collapse`, even when expanded, so it hides the toggle when nothing would collapse.
4. Search field rendered only when `channels.length + playlists.length > SEARCH_THRESHOLD`

**Refresh after an action** (sidebar row, detail header)
1. `refreshing(action)` from `useLibraryAction()` → `await reconcileChannel(id)` / `markChannelWatched(id)` / playlist equivalents
2. → `invalidateLibrary()` → TanStack refetches the active `['channels']` / `['playlists']` queries
3. Delete: the wrapped action is `deleteChannel(id)` → `removeQuery(queryKeys.channelVideos(id))` → `navigate('/')` if it was active; then step 2. Playlists alike.
4. Add dialog: `await createChannel(..)` / `createPlaylist(..)` → `invalidateLibrary()`

**Refresh after playback progress**
1. `useWatchProgress` → `report()` → `recordVideoProgress(videoId, progress)` → `POST /api/videos/<id>/progress`
2. → `record_video_progress` → `VideoWatchStateUpdater::update(&youtube_id, position, reported_duration)` → `Ok(watched)` → `200 RecordProgressResponse { watched }`
3. → client compares `watched` with `lastWatched` (starts as `video.watched`); if different, sets `lastWatched` and calls `queryClient.invalidateQueries({ queryKey: queryKeys.channels, exact: true })`
4. Beacon reports ignore the response.

**Hidden tab**: TanStack's defaults (`refetchIntervalInBackground: false`, `refetchOnWindowFocus: true`) stop intervals while hidden and refetch when visible again. No custom code.

## Test Plan

**Behaviour tests**
1. `it_should_list_channels_sorted_by_name_ignoring_case` (`src/application/http/channels/mod.rs`): insert "veritasium", "Kurzgesagt", "3Blue1Brown" in that order. The response lists them as "3Blue1Brown", "Kurzgesagt", "veritasium".
2. `it_should_list_playlists_sorted_by_name_ignoring_case` (`src/application/http/playlists/mod.rs`): insert "watch later", "Courses", "Ambient". The response lists "Ambient", "Courses", "watch later".
3. `it_should_respond_watched_when_progress_reaches_the_watched_threshold` (`src/application/http/videos/mod.rs`): an unwatched video with a 100s duration gets position 95. Returns `RecordProgressResponse { watched: true }` and the stored video is watched.
4. `it_should_respond_unwatched_when_progress_stays_below_the_watched_threshold`: position 30 returns `RecordProgressResponse { watched: false }` and the stored video keeps position 30.

**Infrastructure tests** (`SqliteChannelVideoRepository`)
5. `it_should_list_every_channel_video`: channel A has 2 channel videos and channel B has 1. Returns all 3 `ChannelVideo`s.

The existing progress tests move from asserting `204` to asserting the response body. The searcher switch to the bulk reads is a refactor guarded by the existing `it_should_count_unwatched_downloaded_videos_when_listing_channels` and `it_should_list_all_channels`. The frontend has no unit test runner, so its behaviour is checked manually against `scripts/run-local.sh`, and the existing smoke tests must stay green.
