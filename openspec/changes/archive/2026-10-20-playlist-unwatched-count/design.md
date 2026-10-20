# Design

## Files

Backend:
- `migrations/0012_playlist_hide_unwatched_count.sql`: adds `playlists.hide_unwatched_count INTEGER NOT NULL DEFAULT 0`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers 0012.
- `src/domain/playlist/playlist.rs`: `hide_unwatched_count` field (`false` on `create`) + `with_hide_unwatched_count`.
- `src/domain/playlist/playlist_settings_change.rs` (new): `PlaylistSettingsChange`. The update now takes two `Option<bool>`s, which are too easy to swap as positional arguments.
- `src/domain/playlist/playlist_view.rs` (new): flat `PlaylistView` = the listed fields plus `unwatched_count`.
- `src/domain/playlist/mod.rs`: exports both.
- `src/domain/services/playlist_view_searcher.rs` (new): `PlaylistViewSearcher`, mirroring `ChannelViewSearcher`.
- `src/domain/services/playlist_searcher.rs`: deleted, since its only caller is `list_playlists`.
- `src/domain/services/mod.rs`: swaps the exports.
- `src/domain/services/playlist_updater.rs`: takes `PlaylistSettingsChange` and applies `hide_unwatched_count`.
- `src/domain/services/video_watch_state_updater.rs`: `mark_playlist_watched`; injects the playlist repository and the playlist video repository. `playlist_videos` and the existing `channel_videos` load their videos with one `find_many` instead of one `find` per entry.
- `src/domain/video/errors.rs`: `UpdateWatchStateError::PlaylistNotFound`.
- `src/domain/services/category_view_searcher.rs`: when counting, drops collected videos whose source is a playlist hiding its count, using the playlists `tracked_members` already loads. A video also collected through a counted member keeps counting once.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs`: reads and writes the new column.
- `src/infrastructure/repositories/sqlite_playlist_video_repository.rs`: `list()` across all playlists.
- `src/application/http/playlists/dto.rs`:
  - `PlaylistResponse` gains `hide_unwatched_count`.
  - New `PlaylistListItemResponse` (the `PlaylistResponse` fields + `unwatched_count`).
  - `UpdatePlaylistRequest` gains `hide_unwatched_count`.
- `src/application/http/playlists/mod.rs`:
  - `list_playlists` uses `PlaylistViewSearcher`.
  - `update_playlist` builds `PlaylistSettingsChange`.
  - New `mark_playlist_watched`.
- `src/application/http/videos/mod.rs`: `update_watch_state_error` maps `PlaylistNotFound` to 400.
- `src/application/http/mod.rs`: `ApiServices.playlist_view_searcher`, and the route `POST /playlists/{id}/watched`.
- `src/serve.rs`: wires `PlaylistViewSearcher` and the new `VideoWatchStateUpdater` dependencies.

Web:
- `web/src/api/types.ts`:
  - `PlaylistListItem` gains `unwatched_count` and `hide_unwatched_count`.
  - `UpdatePlaylistRequest` gains `hide_unwatched_count`.
  - `LibraryItem` gains `hide_unwatched_count?`.
- `web/src/test/helpers.tsx`: `aPlaylist` defaults to `unwatched_count: 0, hide_unwatched_count: false`.
- `web/src/api/client.ts`: `markPlaylistWatched`.
- `web/src/api/queries.ts`: `useSetPlaylistUnwatchedCountHidden`.
- `web/src/lib/sidebarSections.ts`:
  - New `shownUnwatchedCount`.
  - `orderChannels` becomes `orderByUnwatched`, generic over `LibraryItem` and sorting on the shown count.
- `web/src/lib/unwatchedCountSetting.ts` (new): `unwatchedCountSettingAction` for the failure alert, the counterpart of `homeSetting.ts`.
- `web/src/components/EntryActionsMenu.tsx`: "Hide unwatched count" / "Show unwatched count" item after the home item.
- `web/src/components/DetailHeader.tsx`: passes the new props through to `EntryActionsMenu`.
- `web/src/components/PlaylistDetail.tsx`: `unwatchedCount`, `onMarkWatched`, and the hide toggle.
- `web/src/components/Sidebar.tsx`:
  - Orders playlists with `orderByUnwatched`.
  - The row badge uses `shownUnwatchedCount`.
  - The playlist section gets `onMarkWatched` and `onSetUnwatchedCountHidden`.
- `web/src/hooks/useWatchProgress.ts`: when the watched state changes, also refetches `queryKeys.playlists` (`exact`).

Smoke:
- `smoke-tests/tests/playlist.spec.js`: steps for the badge, hiding and showing the count, and "Mark all watched" (covers `POST /playlists/{id}/watched`).

## Types & Signatures

```rust
// domain/playlist/playlist.rs
pub struct Playlist {
    // ...existing fields
    pub hide_unwatched_count: bool,
}
impl Playlist {
    pub fn with_hide_unwatched_count(self, hide_unwatched_count: bool) -> Self;
}

// domain/playlist/playlist_settings_change.rs
/// Settings to change on a playlist; `None` keeps the stored value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaylistSettingsChange {
    pub exclude_from_home: Option<bool>,
    pub hide_unwatched_count: Option<bool>,
    pub reconcile_interval: Option<ReconcileInterval>,
}
impl PlaylistSettingsChange {
    pub fn is_empty(&self) -> bool;
}

// domain/playlist/playlist_view.rs
/// A playlist as listed, with its count of downloaded videos not yet watched.
pub struct PlaylistView {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub kind: PlaylistKind,
    pub exclude_from_home: bool,
    pub hide_unwatched_count: bool,
    pub reconcile_interval: ReconcileInterval,
    pub created_at: DateTime<Utc>,
    pub unwatched_count: usize,
}

// domain/services/playlist_view_searcher.rs
pub struct PlaylistViewSearcher {
    repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
}
pub trait PlaylistViewSearcherApi: Send + Sync {
    fn search_all(&self) -> anyhow::Result<Vec<PlaylistView>>;
}

// domain/services/playlist_updater.rs
pub trait PlaylistUpdaterApi: Send + Sync {
    fn update_settings(
        &self,
        id: PlaylistId,
        change: PlaylistSettingsChange,
    ) -> Result<Playlist, UpdatePlaylistError>;
}

// domain/services/video_watch_state_updater.rs
impl VideoWatchStateUpdater {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        clock: Arc<dyn Clock>,
        category_repository: Arc<dyn CategoryRepository>,
        category_video_collector: CategoryVideoCollector,
    ) -> Self;
}
pub trait VideoWatchStateUpdaterApi: Send + Sync {
    // ...existing
    /// Marks every `Downloaded` video of the playlist watched, with every
    /// stored copy of each; pending/in-flight videos are left unchanged.
    fn mark_playlist_watched(&self, playlist_id: &PlaylistId)
    -> Result<(), UpdateWatchStateError>;
}

// domain/video/errors.rs
pub enum UpdateWatchStateError {
    // ...existing
    PlaylistNotFound(PlaylistId),
}

// infrastructure/repositories/sqlite_playlist_video_repository.rs
pub trait PlaylistVideoRepository: Send + Sync {
    // ...existing
    fn list(&self) -> anyhow::Result<Vec<PlaylistVideo>>;
}

// application/http/playlists/dto.rs
pub struct UpdatePlaylistRequest {
    #[serde(default)] pub exclude_from_home: Option<bool>,
    #[serde(default)] pub hide_unwatched_count: Option<bool>,
    #[serde(default)] pub reconcile_interval_minutes: Option<i64>,
}
pub struct PlaylistResponse { /* ...existing */ pub hide_unwatched_count: bool }
pub struct PlaylistListItemResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub quality: String,
    pub kind: String,
    pub exclude_from_home: bool,
    pub hide_unwatched_count: bool,
    pub reconcile_interval_minutes: u32,
    pub created_at: DateTime<Utc>,
    pub unwatched_count: usize,
}
impl From<PlaylistView> for PlaylistListItemResponse;

// application/http/playlists/mod.rs
const NOTHING_TO_UPDATE: &str =
    "Request must change exclude from home, hide unwatched count or the reconcile interval";
pub async fn list_playlists(State(PlaylistViewSearcher))
    -> Result<Json<Vec<PlaylistListItemResponse>>, ApiError>;
pub async fn mark_playlist_watched(
    State(video_watch_state_updater): State<VideoWatchStateUpdater>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError>;
```

```ts
// api/types.ts
interface PlaylistListItem { /* ...existing */ unwatched_count: number; hide_unwatched_count: boolean }
interface UpdatePlaylistRequest { /* ...existing */ hide_unwatched_count?: boolean }
interface LibraryItem { /* ...existing */ hide_unwatched_count?: boolean }

// api/client.ts
export async function markPlaylistWatched(id: string): Promise<void>

// api/queries.ts
/** Sets whether a playlist's unwatched count is hidden, then refetches the library. */
export function useSetPlaylistUnwatchedCountHidden(): (id: string, hidden: boolean) => Promise<void>

// lib/sidebarSections.ts
/** The unwatched count to display: 0 when the entry hides it. */
export function shownUnwatchedCount(item: LibraryItem): number
/** Entries with a shown unwatched count first, most first; stable otherwise. */
export function orderByUnwatched<T extends LibraryItem>(items: T[]): T[]

// lib/unwatchedCountSetting.ts
export function unwatchedCountSettingAction(hidden: boolean, name: string): string
// -> `hide the unwatched count of "<name>"` / `show the unwatched count of "<name>"`

// components/EntryActionsMenu.tsx (new props)
/** Playlists only: the current setting; undefined hides the item. */
unwatchedCountHidden?: boolean | undefined
onSetUnwatchedCountHidden?: ((hidden: boolean) => Promise<void>) | undefined
```

## Call Stack

List playlists:
```
GET /api/playlists
  list_playlists(State<PlaylistViewSearcher>)
    run_blocking(search_all)
      playlist_repository.list()
      playlist_video_repository.list()
      video_repository.find_many(&[VideoRecordId])
        .filter(Video::is_downloaded_and_unwatched)
      fold -> HashMap<PlaylistId, usize>
      -> PlaylistView { unwatched_count: counts.get(&id).unwrap_or(0), .. }
  -> Vec<PlaylistListItemResponse>
```

Update settings:
```
PATCH /api/playlists/{id} { exclude_from_home?, hide_unwatched_count?, reconcile_interval_minutes? }
  update_playlist
    PlaylistId::new(id)?; ReconcileInterval::new(minutes)?
    change = PlaylistSettingsChange { .. }; change.is_empty() -> 400 NOTHING_TO_UPDATE
    playlist_updater.update_settings(id, change)
      find_playlist -> apply_settings(playlist, change)    // with_* per Some
      repository.update(&playlist)
      change.reconcile_interval.map(bring_forward_next_reconcile)
  -> PlaylistResponse
```

Mark playlist watched:
```
POST /api/playlists/{id}/watched
  mark_playlist_watched
    PlaylistId::new(id)?                                   // 400 on invalid
    video_watch_state_updater.mark_playlist_watched(&id)
      ensure_playlist_exists(&id)                          // PlaylistNotFound -> 400
      playlist_video_repository.list_for_playlist(&id)
        -> video_repository.find_many(&[VideoRecordId])
        .filter(is_downloaded_and_unwatched) -> HashSet<VideoId>
      .try_for_each(mark_copies_watched(youtube_id, now))  // existing helper
  -> 204
```

List categories (counts):
```
GET /api/categories
  category_view_searcher.search_all()
    hidden = playlist_repository.list() where hide_unwatched_count -> HashSet<PlaylistId>
    per category: collector.downloaded_videos(&category)
      .filter(is_downloaded_and_unwatched)
      .filter(source is not VideoSource::Playlist { id } with id in hidden)
      -> HashSet<VideoId>                                // distinct, so a counted copy still counts once
    unwatched_total = distinct union of every category's set
```
"Mark all watched" on a category is unchanged: it still marks hidden playlists' videos, since it's an action, not a counter.

Web, badge + order:
```
Sidebar
  usePlaylists() -> orderByUnwatched(playlists) -> sectionView(.., leadCount: CAUGHT_UP_PREVIEW)
  SidebarRow: shownUnwatchedCount(item) > 0 -> badge
  EntryActionsMenu
    onMarkWatched            = refreshing(markPlaylistWatched)
    onSetUnwatchedCountHidden = useSetPlaylistUnwatchedCountHidden()(id, hidden)
useWatchProgress: watched changed -> invalidate channels (exact) + playlists (exact)
```

## Test Plan

Behaviour tests (Rust):

1. `it_should_count_unwatched_downloaded_videos_when_listing_playlists` (playlists/mod.rs): PL1 has 2 downloaded unwatched videos, 1 downloaded watched, and 1 pending; PL2 has none. The list returns PL1 with `unwatched_count: 2` and PL2 with `0`.
2. `it_should_list_playlists_with_their_hide_unwatched_count`: a playlist stored with `hide_unwatched_count: true` and 1 unwatched video is listed with `hide_unwatched_count: true, unwatched_count: 1`.
3. `it_should_create_a_playlist_showing_its_unwatched_count`: the create response and the stored playlist have `hide_unwatched_count: false`.
4. `it_should_hide_a_playlists_unwatched_count`: PATCH `{hide_unwatched_count: true}` returns and stores `true`, and leaves `exclude_from_home` and the interval unchanged.
5. `it_should_show_a_playlists_unwatched_count_again`: PATCH `{hide_unwatched_count: false}` on a hidden one stores `false`.
6. `it_should_fail_to_update_if_nothing_to_change`: an existing test, updated so its expected message is the new `NOTHING_TO_UPDATE`.
7. `it_should_mark_every_downloaded_playlist_video_watched`: the playlist has a downloaded video, a pending video, and a downloaded video whose YouTube ID is also stored in a channel. Returns 204; both downloaded videos and the channel copy are watched, and the pending video is unchanged.
8. `it_should_fail_to_mark_watched_a_missing_playlist`: 400, nothing changed.
9. `it_should_fail_to_mark_watched_if_invalid_playlist_id_provided`: 400.

Existing `VideoWatchStateUpdater::new` call sites in tests get the two extra repositories. Existing `PlaylistSearcher` list tests move to `PlaylistViewSearcher`, and their expected values become `PlaylistListItemResponse`.

Behaviour tests (web, Vitest; one cycle each):

10. `sidebarSections.test.ts`, `orders entries by shown unwatched count`: hidden counts sort as 0 and ties keep their input order.
11. `sidebarSections.test.ts`, `shows no unwatched count for an entry that hides it`.
12. `Sidebar.test.tsx`, `shows a playlist's unwatched badge unless hidden`.
13. `Sidebar.test.tsx`, `orders playlists by unwatched count`.
14. `Sidebar.test.tsx`, `marks a playlist watched from its row menu` (`POST /api/playlists/PL1/watched`, then the badge disappears).
15. `Sidebar.test.tsx`, `hides and shows a playlist's unwatched count from its row menu` (PATCH body `{hide_unwatched_count: true}`, then the label flips).
16. `Sidebar.test.tsx`, `alerts when hiding a playlist's count fails`.
17. `Sidebar.test.tsx`, `offers playlist row actions in order`: sync, "Mark all watched", home, hide count, "Edit settings", delete. This replaces the "no mark-watched" test.
18. `PlaylistDetail.test.tsx`, `summarizes the playlist's unwatched count`: "3 videos · 2 unwatched", also when the count is hidden.
19. `PlaylistDetail.test.tsx`, `marks the playlist watched from its page header`.
20. `PlaylistDetail.test.tsx`, `hides the unwatched count from its page header`.
21. `useWatchProgress.test.tsx`, `refetches the playlist list when the watched state changes`.
22. `Sidebar.test.tsx` / `ChannelDetail.test.tsx`: channel menus offer no hide count item.

Infrastructure tests:

SQLite playlist repository:

23. `it_should_round_trip_the_playlists_hide_unwatched_count`: insert with `true`, `find` returns `true`; `update` to `false` persists.

SQLite playlist video repository:

24. `it_should_list_videos_of_every_playlist`: rows from PL1 and PL2 are both returned.

Migrations:

25. `it_should_default_existing_playlists_to_show_their_unwatched_count_when_migrating`: a playlist inserted before 0012 reads `hide_unwatched_count = 0`.

Categories (acceptance, categories/mod.rs):

26. `it_should_not_count_videos_of_a_playlist_hiding_its_count_in_a_category`: channel member with 1 unwatched video, hidden playlist member with 2 others; count and `unwatched_total` are 1.
27. `it_should_count_once_a_video_of_a_hidden_playlist_also_in_a_counted_member`: count 1.
28. `it_should_not_total_videos_reached_only_through_hidden_playlists`: count and `unwatched_total` are 0.
