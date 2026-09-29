## Files

**Backend**
- `src/infrastructure/repositories/youtube_playlist_repository.rs`: the port's `exists` becomes `resolve`, which requests `part=snippet,contentDetails` and returns the title and item count. The fake follows the same change.
- `src/domain/playlist/playlist_name.rs`:
  - Drops the filesystem-unsafe character rule and keeps only "not blank".
  - New `from_youtube_title`, which falls back to the ID.
- `src/domain/playlist/playlist_preview.rs` (new): the read model the previewer returns.
- `src/domain/playlist/errors.rs`: new `PreviewPlaylistError`.
- `src/domain/playlist/mod.rs`: exports `PlaylistPreview`.
- `src/domain/services/playlist_creator.rs`: `create` loses `name`. `ensure_exists_on_youtube` becomes `resolve_on_youtube`, which returns the name.
- `src/domain/services/playlist_previewer.rs` (new): one use case, looking a playlist up on YouTube without persisting it.
- `src/domain/services/mod.rs`: exports `PlaylistPreviewer`/`PlaylistPreviewerApi`.
- `src/application/http/playlists/dto.rs`:
  - `CreatePlaylistRequest` drops `name`. Serde ignores unknown fields, so old clients still work.
  - New `PreviewPlaylistQuery` and `PlaylistPreviewResponse`.
- `src/application/http/playlists/mod.rs`: new `preview_playlist` handler. `create_playlist` no longer builds a name.
- `src/application/http/mod.rs`: `ApiServices.playlist_previewer`; route `GET /playlists/preview`.
- `src/serve.rs`: wires `PlaylistPreviewer` with `youtube_playlist_repository`.

**Frontend**
- `web/src/api.js`: `createPlaylist` drops `name`; new `previewPlaylist`.
- `web/src/useDebouncedValue.js` (new): the debounce hook for the lookup.
- `web/src/queries.js`: new `usePlaylistPreview`, under a key outside the `['playlists']` prefix so that `invalidateLibrary` doesn't refetch it.
- `web/src/playlistNotice.js` (new): a pure function that builds the notice's lead sentence (count and title pluralisation), following the pattern of `channelNotice.js`.
- `web/src/components/DestinationNotice.jsx` (new): the notice `<p>` (info/error tone, `data-testid`, "change" action) moved out of `AddChannelDialog.jsx` and shared by both dialogs.
- `web/src/components/AddChannelDialog.jsx`: uses the shared `DestinationNotice`; drops `showDestination={false}`.
- `web/src/components/AddPlaylistDialog.jsx`: rewritten:
  - Controls in order: ID/URL field, notice, "Advanced options" (location, quality), submit.
  - "Advanced options" stays mounted and is toggled with `hidden`, as in the channel dialog.
- `web/src/components/LocationField.jsx`: removes the `showDestination` prop, the "Download destination" box, the creation hints and the conflict line. Only the notices use `onChange`'s `destination`/`occupiedBy` now.

**Smoke tests**
- `smoke-tests/helpers/addDialog.js`:
  - `fillPlaylist`/`submitPlaylist` drop `name`, wait for the notice, and open "Advanced options" before setting the location.
  - `destinationPreview` is removed.
- `smoke-tests/tests/playlist.spec.js`: the sidebar entry is found under the YouTube title. The path-conflict step becomes an already-added check.
- `smoke-tests/tests/addDialogLocation.spec.js`:
  - The playlist cases assert through `destinationNotice`.
  - The channel breadcrumb/descend cases assert through the channel notice.
  - The "Will create new folders" assertion is dropped.
  - The destination-conflict case moves to the channel dialog, which shares the logic.
- `smoke-tests/tests/addPlaylistDialog.spec.js` (new): notice cases.
- `scripts/run-smoke-tests.sh`: the `SMOKE_PLAYLIST_NAME` default becomes the smoke playlist's actual YouTube title.

## Types & Signatures

```rust
// src/infrastructure/repositories/youtube_playlist_repository.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlaylist {
    pub title: String,
    pub item_count: u64,
}

pub trait YoutubePlaylistRepository: Send + Sync {
    fn resolve(&self, id: &PlaylistId) -> anyhow::Result<Option<ResolvedPlaylist>>;
}

#[cfg(test)]
pub struct FakeYoutubePlaylistRepository {
    pub(crate) resolved: Option<ResolvedPlaylist>,
}
```

```rust
// src/domain/playlist/playlist_name.rs
impl PlaylistName {
    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError>; // blank -> Err
    pub fn from_youtube_title(title: &str, id: &PlaylistId) -> Self;      // blank title -> id
}

// src/domain/playlist/playlist_preview.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistPreview {
    pub id: PlaylistId,
    pub name: PlaylistName,
    pub video_count: u64,
}

// src/domain/playlist/errors.rs
#[derive(Debug)]
pub enum PreviewPlaylistError {
    YoutubePlaylistNotFound(PlaylistId), // "YouTube playlist {id} does not exist or is not accessible"
    Lookup(anyhow::Error),
}
```

```rust
// src/domain/services/playlist_creator.rs
pub trait PlaylistCreatorApi: Send + Sync {
    fn create(
        &self,
        id: PlaylistId,
        path: PlaylistPath,
        quality: Quality,
    ) -> Result<CreatePlaylistOutcome, CreatePlaylistError>;
}
// private: fn resolve_on_youtube(&self, id: &PlaylistId) -> Result<PlaylistName, CreatePlaylistError>

// src/domain/services/playlist_previewer.rs
pub struct PlaylistPreviewer {
    lookup: Arc<dyn YoutubePlaylistRepository>,
}

impl PlaylistPreviewer {
    pub fn new(lookup: Arc<dyn YoutubePlaylistRepository>) -> Self;
}

pub trait PlaylistPreviewerApi: Send + Sync {
    fn preview(&self, id: PlaylistId) -> Result<PlaylistPreview, PreviewPlaylistError>;
}
```

```rust
// src/application/http/playlists/dto.rs
#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub playlist: String,
    #[serde(default)] pub path: Option<String>,
    #[serde(default)] pub quality: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewPlaylistQuery {
    #[serde(default)] pub playlist: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PlaylistPreviewResponse {
    pub id: String,
    pub title: String,
    pub video_count: u64,
}
impl From<PlaylistPreview> for PlaylistPreviewResponse;

// src/application/http/playlists/mod.rs
pub async fn preview_playlist(
    State(playlist_previewer): State<PlaylistPreviewer>,
    Query(query): Query<PreviewPlaylistQuery>,
) -> Result<Json<PlaylistPreviewResponse>, ApiError>;
// NotFound -> 404, Lookup -> 502
```

```js
// web/src/api.js
export async function createPlaylist({ playlist, path, quality })
export async function previewPlaylist(playlist) // -> { id, title, video_count }; throws Error(body.error)

// web/src/useDebouncedValue.js
export function useDebouncedValue(value, delayMs) // -> value, after it has been stable for delayMs

// web/src/queries.js
queryKeys.playlistPreview = (value) => ['playlist-preview', value]
export function usePlaylistPreview(value) // useQuery; enabled: Boolean(value), retry: false, staleTime: Infinity

// web/src/playlistNotice.js
// "All 42 videos from “Lofi beats” will be downloaded to"
// "The only video from “Lofi beats” will be downloaded to"   (count 1)
// "Videos from “Lofi beats” will be downloaded to"           (count 0)
export function playlistNoticeLead(videoCount, title) // -> string

// web/src/components/DestinationNotice.jsx
export function DestinationNotice({ tone, children, onChange }) // tone: 'info' | 'error'; onChange optional -> renders [change]

// web/src/components/AddPlaylistDialog.jsx
export function AddPlaylistDialog({ open, onOpenChange })
// emptyForm = { playlist: '', quality: 'high' }

// web/src/components/LocationField.jsx
export function LocationField({ mode, nameSource, onChange })
// onChange payload unchanged: { path, destination, valid, occupiedBy }
```

## Call Stack

**Preview (HTTP)**
1. `GET /api/playlists/preview?playlist=<raw>` → `preview_playlist(State<PlaylistPreviewer>, Query)`
2. → `PlaylistId::from_url_or_id(query.playlist.unwrap_or_default())?`. An absent value is treated as empty, so every invalid value is rejected with the value object's 400 message.
3. → `run_blocking(|| playlist_previewer.preview(id))`
4. → `lookup.resolve(&id)`: `None` → `YoutubePlaylistNotFound`, `Err` → `Lookup`
5. → `PlaylistPreview { id, name: PlaylistName::from_youtube_title(&resolved.title, &id), video_count: resolved.item_count }`
6. → `Json(PlaylistPreviewResponse::from(preview))`

**Create (HTTP)**
1. `POST /api/playlists { playlist, path, quality }` → `create_playlist`
2. → `PlaylistId::from_url_or_id`, `PlaylistPath::new`, `Quality::new`
3. → `playlist_creator.create(id, path, quality)`
4. → `find_existing(&id)`; if the playlist exists → `AlreadyExisted`
5. → `ensure_path_free(&path)`
6. → `resolve_on_youtube(&id)`, which returns a `PlaylistName` via `from_youtube_title`
7. → `Playlist::create(id, name, path, quality, YoutubeLinked, now)` → `insert_and_publish`

**Add playlist dialog**
1. The ID/URL field `onChange` → `form.playlist`
2. → `debounced = useDebouncedValue(form.playlist.trim(), 400)` → `preview = usePlaylistPreview(debounced)`. A query keyed by value never shows a response for older input.
3. → `playlists = usePlaylists()`; `tracked = preview.data && playlists.data?.find((p) => p.id === preview.data.id)`
4. → `<LocationField mode="playlist" nameSource={preview.data?.title ?? ''} onChange={setLocation} />`. It is mounted inside the `hidden` advanced block.
5. The notice is shown when `form.playlist.trim()`. The first matching case wins:
   - `debounced !== form.playlist.trim()` or `preview.isFetching` → info, "Looking up playlist…"
   - `preview.error` → error, `preview.error.message`
   - `tracked` → error, `Already added as “{tracked.name}”`
   - `location.occupiedBy` → error, "`{destination}` is already used by `{occupiedBy}`. Choose a different folder." + `[change]`
   - `location.destination` → info, `playlistNoticeLead(video_count, title)` + `{destination}` + `[change]`
6. `[change]` → `setAdvancedOpen(true)`
7. Submit is disabled unless `preview.data && !tracked && location.valid && !submitting`. It calls `createPlaylist({ playlist: form.playlist, path: location.path, quality })` → `invalidateLibrary()` → reset → `onOpenChange(false)`.

## Test Plan

The frontend has no unit test runner. The dialog is covered by Playwright smoke tests (`scripts/run-smoke-tests.sh`) and checked with `npm run build` and `npm run lint` in `web/`.

**Behaviour tests** (Rust, `src/application/http/playlists/mod.rs`)

The walking skeleton adapts the existing create tests to the request without `name` and to `FakeYoutubePlaylistRepository { resolved }`. `it_should_create_a_playlist` then expects the resolved title (`"Lofi beats"`) as the name, and passes through `from_youtube_title`, which does not fall back yet.

1. `it_should_name_the_playlist_after_its_id_if_youtube_title_blank`: resolved title `"  "`. Response and stored playlist are named `PLabc123`.
2. `it_should_create_a_playlist_with_a_title_unsafe_for_filesystems`: resolved title `"AC/DC: greatest hits?"`. Response and `list()` both carry that name.
3. `it_should_preview_a_playlist`: the fake resolves `{ "Lofi beats", 42 }`. `Ok(PlaylistPreviewResponse { id: "PLabc123", title: "Lofi beats", video_count: 42 })`, and playlist, event and task repositories stay empty.
4. `it_should_preview_a_playlist_from_a_youtube_url`: `watch?v=vid1&list=PLabc123` returns `id: "PLabc123"`.
5. `it_should_fail_to_preview_if_invalid_playlist_provided`: `Err(ApiError::bad_request(<PlaylistId message>))`, on an unmigrated connection.
6. `it_should_fail_to_preview_if_playlist_missing`: `playlist: None` returns `Err(ApiError::bad_request("Playlist ID or URL must not be empty"))`.
7. `it_should_fail_to_preview_if_playlist_not_found_on_youtube`: fake `resolved: None` → `Err(ApiError::new(NOT_FOUND, "YouTube playlist PLabc123 does not exist or is not accessible"))`.
8. `it_should_fail_to_preview_if_youtube_lookup_fails`: a failing fake returns `Err(ApiError::new(BAD_GATEWAY, ..))`.

**Value object tests** (`src/domain/playlist/playlist_name.rs`)
9. `it_should_accept_a_name_with_filesystem_unsafe_characters`: `PlaylistName::new("AC/DC: hits?")` is `Ok`. This replaces the slash and backslash rejection tests.
10. `it_should_name_after_the_id_if_youtube_title_blank`: `from_youtube_title("  ", &id)` → `PlaylistName("PLabc123")`.

**Infrastructure tests** (`youtube_playlist_repository.rs`, mockito)

The walking skeleton adapts the existing "does not exist" test to `resolve`, expecting `None`.

11. `it_should_resolve_the_playlist_title_and_item_count`: the query has `part=snippet,contentDetails` and `id=PLexists`. The body with `snippet.title` and `contentDetails.itemCount` returns `Some(ResolvedPlaylist { title, item_count })`.

**Smoke tests**
12. `addPlaylistDialog.spec.js` › `it should show no notice until a playlist is entered`: the notice is absent and `Create Playlist` is disabled.
13. `addPlaylistDialog.spec.js` › `it should explain a value that is not a playlist`: `https://www.youtube.com/watch?v=abc` → the error notice and a disabled submit.
14. `addPlaylistDialog.spec.js` › `it should state the title, count and destination` (needs `SMOKE_PLAYLIST_ID`): the notice contains the playlist's title, ends with `/playlists/<slug of title>`, and "Advanced options" is collapsed. There is no `Name` field.
15. `addPlaylistDialog.spec.js` › `it should expand advanced options from the change action`: `Folder name` becomes visible with the slug of the title.
16. `playlist.spec.js` › lifecycle: add by ID only. The sidebar entry is the YouTube title. Reopening with the same ID shows "Already added as" with submit disabled.
