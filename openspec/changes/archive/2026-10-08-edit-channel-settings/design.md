## Files

**Backend**
- `src/domain/channel/channel.rs`: `with_quality` / `with_video_limit` transitions.
- `src/domain/channel/channel_view.rs`: `ChannelView` gains `quality` and `video_limit`.
- `src/domain/channel/errors.rs`: new `UpdateChannelError`.
- `src/domain/channel/mod.rs`: re-exports `UpdateChannelError`.
- `src/infrastructure/repositories/sqlite_channel_repository.rs`: new `update` port method (writes `quality` and `video_limit`).
- `src/domain/services/channel_updater.rs` (new): the use case that changes a channel's settings.
- `src/domain/services/mod.rs`: exports `ChannelUpdater` and `ChannelUpdaterApi`.
- `src/domain/services/channel_view_searcher.rs`: fills the two new `ChannelView` fields.
- `src/application/http/channels/dto.rs`: new `UpdateChannelRequest`; `ChannelListItemResponse` gains `quality` and `video_limit`.
- `src/application/http/channels/mod.rs`: new `update_channel` handler and `NOTHING_TO_UPDATE` message.
- `src/application/http/mod.rs`: `ApiServices.channel_updater`; `PATCH /channels/{handle}` route.
- `src/serve.rs`: builds `ChannelUpdater`.
- Fakes: none new; tests use the real SQLite repository on an in-memory DB, like the other channel handler tests.

**Web**
- `web/src/api/types.ts`: `ChannelListItem.quality` and `video_limit`.
- `web/src/test/helpers.tsx`: `aChannel` defaults `quality: 'high'`, `video_limit: 3`.
- `web/src/api/client.ts`: `UpdateChannelRequest`, `updateChannel`.
- `web/src/api/queries.ts`: `useUpdateChannelSettings` (PATCH, then refetch library).
- `web/src/lib/channelSettings.ts` (new): pure helpers `channelSettingsChanges`, `lowersVideoLimit`, unit tested.
- `web/src/components/EditChannelDialog.tsx` (new): the "Edit channel" dialog.
- `web/src/components/EntryActionsMenu.tsx`: optional `onEditRequest` → "Edit settings" item.
- `web/src/components/DetailHeader.tsx`: passes `onEditRequest` through.
- `web/src/components/ChannelDetail.tsx`: owns the dialog's open state, saves, and syncs after a limit change.
- Colocated `*.test.tsx` / `*.test.ts` for each changed component and helper.
- `smoke-tests/tests/channel.spec.js`: edit a channel's settings through the UI (route wiring).

## Types & Signatures

```rust
// domain/channel/channel.rs
impl Channel {
    pub fn with_quality(self, quality: Quality) -> Self;
    pub fn with_video_limit(self, video_limit: VideoLimit) -> Self;
}

// domain/channel/channel_view.rs
pub struct ChannelView {
    pub id: ChannelHandle,
    pub name: String,
    pub path: PlaylistPath,
    pub quality: Quality,
    pub video_limit: VideoLimit,
    pub avatar_filename: Option<String>,
    pub unwatched_count: usize,
}

// domain/channel/errors.rs
pub enum UpdateChannelError {
    NotFound(ChannelHandle),
    Repository(anyhow::Error),
}

// infrastructure/repositories/sqlite_channel_repository.rs
pub trait ChannelRepository: Send + Sync {
    // ...existing
    fn update(&self, channel: &Channel) -> anyhow::Result<()>;
}

// domain/services/channel_updater.rs
#[derive(Clone)]
pub struct ChannelUpdater {
    repository: Arc<dyn ChannelRepository>,
}

pub trait ChannelUpdaterApi: Send + Sync {
    /// `None` keeps the stored value.
    fn update_settings(
        &self,
        id: ChannelHandle,
        quality: Option<Quality>,
        video_limit: Option<VideoLimit>,
    ) -> Result<Channel, UpdateChannelError>;
}

// application/http/channels/dto.rs
pub struct UpdateChannelRequest {
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub video_limit: Option<i64>,
}

pub struct ChannelListItemResponse {
    // ...existing
    pub quality: String,
    pub video_limit: u32,
}

// application/http/channels/mod.rs
const NOTHING_TO_UPDATE: &str = "Request must change the quality or the video limit";

pub async fn update_channel(
    State(channel_updater): State<ChannelUpdater>,
    Path(handle): Path<String>,
    Json(request): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError>;
```

```ts
// web/src/api/types.ts
export interface ChannelListItem { /* ...existing */ quality: VideoQuality; video_limit: number }

// web/src/api/client.ts
export interface UpdateChannelRequest { quality?: string; video_limit?: number }
export function updateChannel(handle: string, body: UpdateChannelRequest): Promise<CreatedChannel>

// web/src/api/queries.ts
export function useUpdateChannelSettings(): (handle: string, changes: UpdateChannelRequest) => Promise<void>

// web/src/lib/channelSettings.ts
export interface ChannelSettingsForm { quality: string; video_limit: string }
export function channelSettingsChanges(
  current: Pick<ChannelListItem, 'quality' | 'video_limit'>,
  form: ChannelSettingsForm,
): UpdateChannelRequest            // only the changed fields; {} when nothing changed
export function lowersVideoLimit(currentLimit: number, entered: string): boolean

// web/src/components/EditChannelDialog.tsx
interface EditChannelDialogProps {
  channel: ChannelListItem
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Sends the changes; rejects to keep the dialog open with the error. */
  onSave: (changes: UpdateChannelRequest) => Promise<void>
}

// web/src/components/EntryActionsMenu.tsx (added prop)
onEditRequest?: () => void        // channels' page header only; shows "Edit settings"

// web/src/components/DetailHeader.tsx (added prop)
onEditRequest?: () => void
```

## Call Stack

**Update**
```
PATCH /channels/{handle} {quality?, video_limit?}
  update_channel(State<ChannelUpdater>, Path(handle), Json<UpdateChannelRequest>)
    ChannelHandle::new(handle)?
    quality = request.quality.map(Quality::new).transpose()?
    video_limit = request.video_limit.map(VideoLimit::new).transpose()?
    both None => 400 NOTHING_TO_UPDATE
    ChannelUpdater::update_settings(id, quality, video_limit)
      ChannelRepository::find(&id) -> None => NotFound (404)
      channel.with_quality(q) / .with_video_limit(l) for each Some
      ChannelRepository::update(&channel)
    -> Json(ChannelResponse)
```

**List**
```
GET /channels
  list_channels -> ChannelViewSearcher::search_all
    channel_view(channel, counts) copies quality and video_limit
  -> ChannelListItemResponse { ..., quality, video_limit }
```

**Web**
```
ChannelDetail
  DetailHeader onEditRequest -> EntryActionsMenu "Edit settings" -> setEditing(true)
  EditChannelDialog(channel, onSave)
    submit: changes = channelSettingsChanges(channel, form)
      {} -> close, no request
      else await onSave(changes) -> close   (reject -> show error, stay open)
  onSave(changes):
    await useUpdateChannelSettings()(id, changes)   // updateChannel + invalidate library
    if changes.video_limit !== undefined:
      refreshing(() => reconcileChannel(id))() not awaited; failure -> window.alert
```

## Test Plan

**1. Behaviour / acceptance tests (Rust, `application/http/channels/mod.rs`)**
1. `it_should_update_a_channels_quality`: stored `high`, PATCH `{quality: "low"}` → 200 with `low`; stored channel equals original `with_quality(Low)`.
2. `it_should_update_a_channels_video_limit`: stored 5, PATCH `{video_limit: 20}` → 200 with 20; stored equals original `with_video_limit(20)`.
3. `it_should_update_both_settings_of_a_channel`: PATCH both → both stored and returned.
4. `it_should_leave_a_channel_unchanged_if_settings_already_set`: PATCH current values → 200, storage unchanged.
5. `it_should_fail_to_update_an_unknown_channel`: → 404 `channel @x not found`, nothing stored.
6. `it_should_fail_to_update_if_nothing_to_update`: `{}` → 400 `NOTHING_TO_UPDATE`.
7. `it_should_fail_to_update_if_invalid_quality_provided`: → 400 with the `Quality` validation message, storage unchanged.
8. `it_should_fail_to_update_if_invalid_video_limit_provided`: 1001 → 400 with the `VideoLimit` range message, storage unchanged.
9. `it_should_fail_to_update_if_invalid_handle_provided`: `somechannel` → 400 with the `ChannelHandle` message.
10. `it_should_list_channels_with_their_quality_and_video_limit`: list response carries each channel's quality and limit.

**2. Infrastructure tests (`infrastructure/repositories/sqlite_channel_repository.rs`)**
11. `it_should_update_an_existing_channel`: insert, `update` with new quality and limit → `find` returns them, other fields unchanged.

**3. Web (Vitest, colocated)**
12. `lib/channelSettings`: `channelSettingsChanges` returns only changed fields / `{}`; `lowersVideoLimit` true only below current.
13. `EditChannelDialog` "opens prefilled with the channel's current settings".
14. `EditChannelDialog` "sends only the changed quality".
15. `EditChannelDialog` "sends nothing and closes when nothing changed".
16. `EditChannelDialog` "warns when lowering the video limit" / no warning when raising.
17. `EditChannelDialog` "does not submit an out-of-range video limit".
18. `EditChannelDialog` "keeps the dialog open and shows the error when saving fails".
19. `ChannelDetail` "edits the channel's settings from its page header menu": PATCH `{quality}` routed, no reconcile.
20. `ChannelDetail` "syncs the channel after its video limit changes": PATCH then POST reconcile.
21. `ChannelDetail` "alerts when the sync after saving fails".
22. `PlaylistDetail` "offers no Edit settings item".

**4. Smoke (Playwright)**
23. Edit a channel's video quality through its page header "⋮" menu; reopening the dialog shows the new value (covers `PATCH /channels/{handle}` wiring).
