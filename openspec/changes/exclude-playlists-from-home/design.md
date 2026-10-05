## Files

**Backend**
- `migrations/0007_playlist_exclude_from_home.sql`: adds `playlists.exclude_from_home INTEGER NOT NULL DEFAULT 0`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers migration 0007.
- `src/domain/playlist/playlist.rs`: `Playlist` gains `exclude_from_home`, `create` takes it, and a new `with_exclude_from_home` transition.
- `src/domain/playlist/errors.rs`: new `UpdatePlaylistError`.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs`: reads and writes the column; new `update` port method.
- `src/domain/services/playlist_creator.rs`: `create` takes `exclude_from_home`.
- `src/domain/services/playlist_updater.rs` (new): the use case that changes a playlist's exclude-from-home setting.
- `src/domain/services/mod.rs`: exports `PlaylistUpdater` and `PlaylistUpdaterApi`.
- `src/domain/services/video_searcher.rs`: `downloaded_from_playlists` skips excluded playlists.
- `src/application/http/playlists/dto.rs`: `CreatePlaylistRequest.exclude_from_home`, new `UpdatePlaylistRequest`, `PlaylistResponse.exclude_from_home`.
- `src/application/http/playlists/mod.rs`: `create_playlist` passes the flag; new `update_playlist` handler.
- `src/application/http/videos/mod.rs`: home acceptance tests for exclusion.
- `src/application/http/mod.rs`: `ApiServices.playlist_updater`; `PATCH /playlists/{id}` route.
- `src/serve.rs`: builds `PlaylistUpdater`.
- Every other `Playlist::create(..)` call site (tests): passes `false`.

**Web**
- `web/src/api/types.ts`: `PlaylistListItem.exclude_from_home`.
- `web/src/test/helpers.tsx`: `aPlaylist` defaults `exclude_from_home: false`.
- `web/src/api/client.ts`: `CreatePlaylistRequest.exclude_from_home`; new `updatePlaylist`.
- `web/src/api/queries.ts`: new `useSetPlaylistExcludedFromHome` hook (invalidates library + home).
- `web/src/components/AddPlaylistDialog.tsx`: "Exclude from home" checkbox under Advanced options.
- `web/src/components/EntryActionsMenu.tsx` (new): shared "⋮" menu (mark watched / exclude-include home / delete), used by the sidebar row and the detail header.
- `web/src/components/Sidebar.tsx`: `SidebarRowMenu` adds the exclude/include item (keeps Sync in its menu).
- `web/src/components/DetailHeader.tsx`: visible Sync button + `EntryActionsMenu`.
- `web/src/components/PlaylistDetail.tsx` / `ChannelDetail.tsx`: pass the new props.
- `web/src/components/VideoActionsMenu.tsx`: optional exclude-playlist item.
- `web/src/components/Home.tsx`: passes the card's playlist source to `VideoActionsMenu`.
- Colocated `*.test.tsx` for each changed component.
- `smoke-tests/tests/playlist.spec.js`: exercises `PATCH /playlists/{id}` through the UI (route wiring).

## Types & Signatures

```rust
// domain/playlist/playlist.rs
pub struct Playlist {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub kind: PlaylistKind,
    pub exclude_from_home: bool,
    pub created_at: DateTime<Utc>,
}

impl Playlist {
    pub fn create(
        id: PlaylistId,
        name: PlaylistName,
        path: PlaylistPath,
        quality: Quality,
        kind: PlaylistKind,
        exclude_from_home: bool,
        created_at: DateTime<Utc>,
    ) -> Self;
    pub fn with_exclude_from_home(self, exclude_from_home: bool) -> Self;
}

// domain/playlist/errors.rs
pub enum UpdatePlaylistError {
    NotFound(PlaylistId),
    Repository(anyhow::Error),
}

// infrastructure/repositories/sqlite_playlist_repository.rs
pub trait PlaylistRepository: Send + Sync {
    // ...existing
    fn update(&self, playlist: &Playlist) -> anyhow::Result<()>;
}

// domain/services/playlist_creator.rs
pub trait PlaylistCreatorApi: Send + Sync {
    fn create(
        &self,
        id: PlaylistId,
        path: PlaylistPath,
        quality: Quality,
        exclude_from_home: bool,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError>;
}

// domain/services/playlist_updater.rs
#[derive(Clone)]
pub struct PlaylistUpdater {
    repository: Arc<dyn PlaylistRepository>,
}

pub trait PlaylistUpdaterApi: Send + Sync {
    fn update_exclude_from_home(
        &self,
        id: PlaylistId,
        exclude_from_home: bool,
    ) -> Result<Playlist, UpdatePlaylistError>;
}

// application/http/playlists/dto.rs
pub struct CreatePlaylistRequest {
    pub playlist: String,
    pub path: Option<String>,
    pub quality: Option<String>,
    #[serde(default)]
    pub exclude_from_home: Option<bool>,
}

pub struct UpdatePlaylistRequest {
    #[serde(default)]
    pub exclude_from_home: Option<bool>,
}

pub struct PlaylistResponse {
    // ...existing
    pub exclude_from_home: bool,
}

// application/http/playlists/mod.rs
pub async fn update_playlist(
    State(playlist_updater): State<PlaylistUpdater>,
    Path(id): Path<String>,
    Json(request): Json<UpdatePlaylistRequest>,
) -> Result<Json<PlaylistResponse>, ApiError>;
```

```ts
// web/src/api/client.ts
export function updatePlaylist(id: string, body: { exclude_from_home: boolean }): Promise<PlaylistListItem>

// web/src/api/queries.ts
export function useSetPlaylistExcludedFromHome(): (id: string, excluded: boolean) => Promise<void>

// web/src/components/EntryActionsMenu.tsx
interface EntryActionsMenuProps {
  name: string
  onMarkWatched?: () => Promise<void>       // channels only
  excludedFromHome?: boolean                // playlists only; undefined hides the item
  onSetExcludedFromHome?: (excluded: boolean) => Promise<void>
  onDeleteRequest: () => void
  className?: string
}

// web/src/components/VideoActionsMenu.tsx (added prop)
excludablePlaylist?: { id: string; name: string }   // set only by Home for playlist sources
```

## Call Stack

**Create**
```
POST /playlists {playlist, path, quality, exclude_from_home?}
  create_playlist(State<PlaylistCreator>, Json<CreatePlaylistRequest>)
    exclude_from_home = request.exclude_from_home.unwrap_or(false)
    PlaylistCreator::create(id, path, quality, exclude_from_home)
      Playlist::create(id, name, path, quality, YoutubeLinked, exclude_from_home, now)
      PlaylistRepository::insert(&playlist)
```

**Update**
```
PATCH /playlists/{id} {exclude_from_home}
  update_playlist(State<PlaylistUpdater>, Path(id), Json<UpdatePlaylistRequest>)
    PlaylistId::new(id)?
    required(request.exclude_from_home, MISSING_EXCLUDE_FROM_HOME)?
    PlaylistUpdater::update_exclude_from_home(id, exclude_from_home)
      PlaylistRepository::find(&id) -> None => NotFound (404)
      playlist.with_exclude_from_home(exclude_from_home)
      PlaylistRepository::update(&playlist)
    -> Json(PlaylistResponse)
```

**Home**
```
GET /videos/home
  list_home_videos(State<VideoSearcher>)
    VideoSearcher::list_home(HOME_LIMITS)
      downloaded_across_sources()
        downloaded_from_channels()
        downloaded_from_playlists()
          PlaylistRepository::list()
          .filter(|playlist| !playlist.exclude_from_home)
          ...as today
```

**Web toggles**
```
Home card ⋮ "Exclude "<name>" from home"   --+
Sidebar row ⋮ "Exclude/Include in home"      +--> useSetPlaylistExcludedFromHome(id, excluded)
DetailHeader ⋮ "Exclude/Include in home"   --+       updatePlaylist(id, {exclude_from_home})
                                                     invalidate playlists + channels + recentVideos
```

## Test Plan

**1. Behaviour / acceptance tests (Rust, application layer)**

`application/http/playlists/mod.rs`
1. `it_should_create_a_playlist_shown_on_home_by_default`: no `exclude_from_home` → 201 response with `exclude_from_home: false`, stored playlist matches.
2. `it_should_create_a_playlist_excluded_from_home`: `exclude_from_home: Some(true)` → response and stored playlist have `true`.
3. `it_should_keep_exclude_from_home_of_an_existing_playlist_on_duplicate_create`: stored `false`, request `true` → 200 with the existing record, unchanged in storage.
4. `it_should_list_playlists_with_their_exclude_from_home`: seeded mix → list response carries each flag.
5. `it_should_exclude_a_playlist_from_home`: stored `false`, PATCH `true` → 200 with `true`; stored playlist equals the original `with_exclude_from_home(true)`.
6. `it_should_include_a_playlist_in_home_again`: stored `true`, PATCH `false` → `false` returned and stored.
7. `it_should_leave_a_playlist_unchanged_if_already_set`: stored `true`, PATCH `true` → 200, storage unchanged.
8. `it_should_fail_to_update_an_unknown_playlist`: → `Err(ApiError 404 "playlist <id> not found")`, nothing stored.
9. `it_should_fail_to_update_if_exclude_from_home_missing`: → `Err(ApiError::bad_request(MISSING_EXCLUDE_FROM_HOME))` (unmigrated `any_playlist_updater()`).
10. `it_should_fail_to_update_if_invalid_id_provided`: → bad request with the `PlaylistId` validation message.

`application/http/videos/mod.rs`
11. `it_should_leave_videos_of_a_playlist_excluded_from_home_out_of_home`: an excluded playlist has a long, a short and an in-progress downloaded video → `empty_home()`.
12. `it_should_show_a_video_of_an_excluded_playlist_through_another_source_on_home`: the same video in an excluded playlist and a shown playlist → listed once under latest, with the shown playlist as source.
13. `it_should_fill_home_with_shown_playlists_if_an_excluded_one_has_newer_videos`: excluded playlist synced later than the shown one → only the shown playlist's videos, in order.

**2. Infrastructure tests**

`infrastructure/repositories/sqlite_playlist_repository.rs`
14. `it_should_return_the_exclude_from_home_of_an_inserted_playlist`: insert with `true` → `find` returns it with `true`.
15. `it_should_update_an_existing_playlist`: insert, `update` with `with_exclude_from_home(true)` → `find` returns the updated playlist, other fields unchanged.

`infrastructure/shared/sqlite_migrations.rs`
16. `it_should_default_existing_playlists_to_shown_on_home_when_migrating`: a playlist row inserted before 0007 → after migrating, `exclude_from_home = 0`.

**3. Web (Vitest, colocated)**
17. `AddPlaylistDialog` "sends exclude_from_home false by default".
18. `AddPlaylistDialog` "sends exclude_from_home true when "Exclude from home" is checked": checkbox under Advanced options.
19. `VideoActionsMenu` "excludes the playlist from home when its item is chosen": PATCH `{exclude_from_home: true}` routed, home and library refetched.
20. `VideoActionsMenu` "offers no exclude item without a playlist".
21. `VideoActionsMenu` "alerts and leaves the playlist as it was when excluding fails".
22. `Home` "excluding a card's playlist from home removes its cards".
23. `Home` "a channel card offers no exclude item".
24. `Sidebar` "excludes a playlist from home from its row menu": label flips to "Include in home".
25. `Sidebar` "includes an excluded playlist in home from its row menu": PATCH `{exclude_from_home: false}`.
26. `Sidebar` "a channel row menu offers no home item".
27. `Sidebar` "alerts when changing a playlist's home setting fails".
28. `PlaylistDetail` "keeps Sync visible and the other actions in the ⋮ menu".
29. `PlaylistDetail` "excludes the playlist from home from its page header menu".
30. `PlaylistDetail` "deletes the playlist from its page header menu after confirming".
31. `ChannelDetail` "marks the channel watched from its page header menu".

**4. Smoke (Playwright)**
32. `smoke-tests/tests/playlist.spec.js`: exclude the playlist from home through its ⋮ menu → its card leaves home; include it again → the card returns (covers the `PATCH /playlists/{id}` route).
