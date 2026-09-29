## Files

**Backend**
- `src/infrastructure/repositories/sqlite_channel_repository.rs`: `list` orders by `name COLLATE NOCASE, rowid`.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs`: `list` orders by `name COLLATE NOCASE, rowid`.
- `src/infrastructure/repositories/sqlite_channel_video_repository.rs`: new `list`, every stored `ChannelVideo` across all channels in one query.
- `src/domain/services/channel_view_searcher.rs`: counts in Rust from 3 bulk reads (channels, all channel videos, `VideoRepository::find_many`) instead of one lookup per video. The downloaded-and-unwatched rule stays in the domain.
- `src/application/http/channels/mod.rs`, `src/application/http/playlists/mod.rs`: new ordering tests.

**Frontend**
- `web/package.json`: add `@tanstack/react-query`.
- `web/src/main.jsx`: create the `QueryClient` and wrap `App` in `QueryClientProvider`.
- `web/src/queries.js` (new): query keys, one hook per fetched resource with its refresh interval, and `useInvalidateLibrary`.
- `web/src/sidebarSections.js` (new): pure functions that order, collapse and filter sidebar rows, plus the thresholds (10 / 5 / 15).
- `web/src/usePolling.js`: deleted, replaced by `queries.js`.
- `web/src/components/Sidebar.jsx`: search field, collapse controls, remembered expanded state; uses `useChannels`/`usePlaylists` and invalidates after actions.
- `web/src/components/ChannelDetail.jsx`, `PlaylistDetail.jsx`: share the list queries; invalidate after sync, mark watched and delete.
- `web/src/components/Home.jsx`, `TasksView.jsx`: `useRecentVideos` / `useTasks` instead of `usePolling`.
- `web/src/components/AddDialog.jsx`: invalidate the lists after a successful add.
- `web/src/useWatchProgress.js`: invalidate channels after each successful (non-beacon) progress report.

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
    fn channel_view(&self, channel: Channel, counts: &HashMap<ChannelHandle, usize>) -> ChannelView;
}
// `count_unwatched` (per-video `find`) is removed.
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
export function matchesSearch(item, text)                        // -> case-insensitive name contains
```

## Call Stack

**List channels**: `GET /api/channels`
1. `list(State<ChannelViewSearcher>)`
2. → `ChannelViewSearcher::search_all()`
   - → `ChannelRepository::list()` (sorted by name, NOCASE)
   - → `unwatched_counts()`
     - → `ChannelVideoRepository::list()` (all channel videos)
     - → `VideoRepository::find_many(&video_ids)` (one `IN (...)` query)
     - → keep videos with `status == Downloaded && !is_watched()`, then fold the channel videos into `HashMap<ChannelHandle, usize>`
   - → each `Channel` → `channel_view(channel, &counts)` with `unwatched_count: counts.get(&id).unwrap_or(0)`
3. → `ChannelListItemResponse::from` for each

**List playlists**: `GET /api/playlists` → unchanged path → `PlaylistRepository::list()` (sorted by name, NOCASE).

**Render sidebar**
1. `Sidebar` → `useChannels()`, `usePlaylists()`, search text state, expanded state per section (localStorage `yarrtube.sidebar.expanded.<section>`, read/write in try/catch)
2. Channels: `orderChannels(channels)` → searching ? `filter(matchesSearch)` : expanded ? all : `collapse(ordered, channelLeadCount(ordered), activeChannelId)`
3. Playlists: searching ? `filter(matchesSearch)` : expanded ? all : `collapse(playlists, CAUGHT_UP_PREVIEW, activePlaylistId)`
4. Search field rendered only when `channels.length + playlists.length > SEARCH_THRESHOLD`

**Refresh after an action** (sidebar row, detail header, add dialog)
1. `await reconcileChannel(id)` / `markChannelWatched(id)` / `deleteChannel(id)` / `createChannel(..)` / playlist equivalents
2. → `invalidateLibrary()` → TanStack refetches the active `['channels']` / `['playlists']` queries

**Refresh after playback progress**
1. `useWatchProgress` → `report()` → `recordVideoProgress(videoId, progress)`
2. → on success `queryClient.invalidateQueries({ queryKey: queryKeys.channels })`

**Hidden tab**: TanStack's defaults (`refetchIntervalInBackground: false`, `refetchOnWindowFocus: true`) stop intervals while hidden and refetch when visible again. No custom code.

## Test Plan

**Behaviour tests**
1. `it_should_list_channels_sorted_by_name_ignoring_case` (`src/application/http/channels/mod.rs`): insert "veritasium", "Kurzgesagt", "3Blue1Brown" in that order. The response lists them as "3Blue1Brown", "Kurzgesagt", "veritasium".
2. `it_should_list_playlists_sorted_by_name_ignoring_case` (`src/application/http/playlists/mod.rs`): insert "watch later", "Courses", "Ambient". The response lists "Ambient", "Courses", "watch later".

**Infrastructure tests** (`SqliteChannelVideoRepository`)
3. `it_should_list_every_channel_video`: channel A has 2 channel videos and channel B has 1. Returns all 3 `ChannelVideo`s.

The searcher switch to the bulk reads is a refactor guarded by the existing `it_should_count_unwatched_downloaded_videos_when_listing_channels` and `it_should_list_all_channels`. The frontend has no unit test runner, so its behaviour is checked manually against `scripts/run-local.sh`, and the existing smoke tests must stay green.
