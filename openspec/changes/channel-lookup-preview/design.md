## Files

**Backend**
- `src/domain/channel/channel_preview.rs` (new): the read model the previewer returns.
- `src/domain/channel/errors.rs`: new `PreviewChannelError`.
- `src/domain/channel/mod.rs`: exports `ChannelPreview` and `PreviewChannelError`.
- `src/domain/services/channel_previewer.rs` (new): one use case, looking a channel up on YouTube without persisting it or storing its avatar.
- `src/domain/services/mod.rs`: exports `ChannelPreviewer`/`ChannelPreviewerApi`.
- `src/application/http/channels/dto.rs`: new `PreviewChannelQuery` and `ChannelPreviewResponse`.
- `src/application/http/channels/mod.rs`: new `preview_channel` handler.
- `src/application/http/mod.rs`: `ApiServices.channel_previewer`; route `GET /channels/preview`, registered before `/channels/{handle}` (axum prefers the static segment either way, but keeping it first reads right).
- `src/serve.rs`: wires `ChannelPreviewer` with `youtube_channel_repository`.

- `src/infrastructure/repositories/youtube_channel_repository.rs`: `ChannelsResponse.items` gets `#[serde(default)]`. YouTube's channels endpoint leaves `items` out entirely when a handle matches nothing (the reply has only `kind`, `etag` and `pageInfo`), so without the default an unknown channel fails to parse and becomes `Lookup` (502) instead of `None` (404). `resolve` otherwise already returns the title and avatar URL. The playlists endpoint does return `"items": []`, so the playlist adapter is unchanged.

**Frontend**
- `web/src/api.js`: new `previewChannel`.
- `web/src/queries.js`: new `useChannelPreview`, under a key outside the `['channels']` prefix so that `invalidateLibrary` doesn't refetch it.
- `web/src/channelNotice.js`: `channelNoticeLead` takes the channel's title.
- `web/src/components/AddChannelDialog.jsx`:
  - Runs the debounced lookup with the same case order as `AddPlaylistDialog`.
  - Renders the avatar through `Thumbnail`, which already falls back to a placeholder on a missing or broken URL.
  - Gates submit on the preview.
- `web/src/components/DestinationNotice.jsx`: optional `leading` slot rendered before the text, for the avatar.

**Smoke tests**
- `smoke-tests/helpers/addDialog.js`: `fillChannel` waits for the lookup to settle, as `fillPlaylist` does.
- `smoke-tests/tests/addChannelDialog.spec.js`:
  - Existing cases move from `@some-handle` to `SMOKE_CHANNEL_HANDLE`.
  - New lookup cases.
- `smoke-tests/tests/addDialogLocation.spec.js`: the channel cases (breadcrumb, descend, destination conflict) use `SMOKE_CHANNEL_HANDLE`.
- `smoke-tests/tests/channel.spec.js`:
  - The re-add step expects "Already added as".
  - The invalid-handle step asserts the error notice and a disabled submit instead of submitting.

## Types & Signatures

```rust
// src/domain/channel/channel_preview.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelPreview {
    pub id: ChannelHandle,
    pub name: String,
    pub avatar_url: Option<String>,
}

// src/domain/channel/errors.rs
#[derive(Debug)]
pub enum PreviewChannelError {
    YoutubeChannelNotFound(ChannelHandle), // "YouTube channel {handle} does not exist or is not accessible"
    Lookup(anyhow::Error),
}
```

```rust
// src/domain/services/channel_previewer.rs
#[derive(Clone)]
pub struct ChannelPreviewer {
    lookup: Arc<dyn YoutubeChannelRepository>,
}

impl ChannelPreviewer {
    pub fn new(lookup: Arc<dyn YoutubeChannelRepository>) -> Self;
}

pub trait ChannelPreviewerApi: Send + Sync {
    fn preview(&self, id: ChannelHandle) -> Result<ChannelPreview, PreviewChannelError>;
}
// private: fn resolve_on_youtube(&self, id: &ChannelHandle) -> Result<ResolvedChannel, PreviewChannelError>
```

```rust
// src/application/http/channels/dto.rs
#[derive(Debug, Deserialize)]
pub struct PreviewChannelQuery {
    #[serde(default)] pub channel: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ChannelPreviewResponse {
    pub id: String,
    pub title: String,
    pub avatar_url: Option<String>,
}
impl From<ChannelPreview> for ChannelPreviewResponse;

// src/application/http/channels/mod.rs
pub async fn preview_channel(
    State(channel_previewer): State<ChannelPreviewer>,
    Query(query): Query<PreviewChannelQuery>,
) -> Result<Json<ChannelPreviewResponse>, ApiError>;
// YoutubeChannelNotFound -> 404, Lookup -> 502
```

```js
// web/src/api.js
export async function previewChannel(channel) // -> { id, title, avatar_url }; throws Error(body.error)

// web/src/queries.js
queryKeys.channelPreview = (value) => ['channel-preview', value]
export function useChannelPreview(value) // useQuery; enabled: Boolean(value), retry: false, staleTime: Infinity

// web/src/channelNotice.js
// "The latest 3 videos from “Veritasium” will be downloaded to"
// "The latest video from “Veritasium” will be downloaded to"     (limit 1)
// "Videos from “Veritasium” will be downloaded to"               (limit out of range)
export function channelNoticeLead(videoLimit, title) // -> string

// web/src/components/DestinationNotice.jsx
export function DestinationNotice({ tone, leading, children, onChange }) // leading: optional node before the text

// web/src/components/AddChannelDialog.jsx
export function AddChannelDialog({ open, onOpenChange })
// emptyForm unchanged: { channel: '', quality: 'high', video_limit: '3' }
```

## Call Stack

**Preview (HTTP)**
1. `GET /api/channels/preview?channel=<raw>` → `preview_channel(State<ChannelPreviewer>, Query)`
2. → `ChannelHandle::from_url_or_handle(query.channel.unwrap_or_default())?`: an absent value is treated as empty (400 with the value object's message).
3. → `run_blocking(|| channel_previewer.preview(id))`
4. → `lookup.resolve(&id)`: `None` → `YoutubeChannelNotFound`, `Err` → `Lookup`
5. → `ChannelPreview { id, name: resolved.title, avatar_url: resolved.avatar_url }`
6. → `Json(ChannelPreviewResponse::from(preview))`

**Add channel dialog**
1. The handle/URL field `onChange` → `form.channel`
2. → `debounced = useDebouncedValue(form.channel.trim(), 400)` → `preview = useChannelPreview(debounced)`
3. → `channels = useChannels()`; `tracked = preview.data && channels.data?.find((channel) => sameHandle(channel.id, preview.data.id))`. Handles compare case-insensitively, since YouTube treats `@Name` and `@name` as the same channel.
4. → `<LocationField mode="channel" nameSource={deriveChannelPathSegment(form.channel)} …>`: unchanged, still derived from the typed handle.
5. The notice is shown when `form.channel.trim()`. The first matching case wins:
   - `debounced !== form.channel.trim()` or `preview.isFetching` → info, "Looking up channel…"
   - `preview.error` → error, `preview.error.message`
   - `tracked` → error, `Already added as “{tracked.name}”`
   - `location.occupiedBy` → error, existing text + `[change]`
   - `location.destination` → info, `leading=<Thumbnail src={avatar_url} …/>`, then `channelNoticeLead(video_limit, title)` + `{destination}` + `[change]`
6. Submit is disabled unless `preview.data && !tracked && !lookingUp && location.valid && !submitting`.

## Test Plan

The frontend has no unit test runner. The dialog is covered by Playwright smoke tests and checked with `npm run build` and `npm run lint` in `web/`.

**Behaviour tests** (Rust, `src/application/http/channels/mod.rs`)

The walking skeleton's previewer returns `Err(YoutubeChannelNotFound(id))`.

1. `it_should_preview_a_channel`: the fake resolves `{ "Some Channel", avatar }`. Returns `Ok(ChannelPreviewResponse { id: "@somechannel", title: "Some Channel", avatar_url: Some(..) })`. The channel, event and task repositories stay empty. The previewer has no avatar store, so nothing can be stored.
2. `it_should_preview_a_channel_from_a_youtube_url`: `https://www.youtube.com/@somechannel/videos` returns `id: "@somechannel"`.
3. `it_should_preview_a_channel_without_an_avatar`: resolved `avatar_url: None` → `avatar_url: None`.
4. `it_should_fail_to_preview_if_invalid_channel_provided`: `somechannel` (no `@`) → `Err(ApiError::bad_request(<ChannelHandle message>))`. Uses `any_channel_previewer()`, whose lookup fails, so a request that wrongly reaches YouTube returns 502 instead.
5. `it_should_fail_to_preview_if_channel_missing`: `channel: None` → `Err(ApiError::bad_request("Channel handle or URL must not be empty"))`.
6. `it_should_fail_to_preview_if_channel_not_found_on_youtube`: `resolved: None` → `Err(ApiError::new(NOT_FOUND, "YouTube channel @somechannel does not exist or is not accessible"))`.
7. `it_should_fail_to_preview_if_youtube_lookup_fails`: a failing lookup → `Err(ApiError::new(BAD_GATEWAY, ..))`.

**Infrastructure tests** (`youtube_channel_repository.rs`, mockito)
- `it_should_return_none_when_youtube_omits_items`: a 200 reply of `{"kind": "youtube#channelListResponse", "etag": "…", "pageInfo": {"totalResults": 0, "resultsPerPage": 5}}` resolves to `None`.

**Smoke tests** (need `SMOKE_CHANNEL_HANDLE`)
8. `addChannelDialog.spec.js` › `it should explain a value that is not a channel`: `somechannel` → an error notice with the `@` message, and submit disabled.
9. `addChannelDialog.spec.js` › `it should state the video limit, title and destination for a handle`: the notice contains `“<title>”`, where the title is read from `GET /api/channels/preview`. It shows the avatar, either an `img` or `Thumbnail`'s placeholder (YouTube's image host can fail, and the placeholder is the intended fallback). When an `img` renders, its `src` equals the preview's `avatar_url`. Its path ends with `/channels/<slug of handle>`.
10. `addChannelDialog.spec.js` › `it should report a channel YouTube doesn't know`: a random handle → an error notice with "does not exist", and submit disabled.
11. `channel.spec.js` › lifecycle:
    - Re-adding shows "Already added as" with submit disabled.
    - The invalid-handle step asserts the error notice instead of submitting.
