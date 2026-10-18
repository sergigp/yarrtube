## Files

Backend:
- `migrations/0009_reconcile_interval.sql` (new): adds `reconcile_interval_minutes INTEGER NOT NULL DEFAULT 180` to `channels` and `playlists`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers 0009.
- `src/domain/shared/reconcile_interval.rs` (new): `ReconcileInterval` VO, shared by channels and playlists like `Quality`.
- `src/domain/shared/mod.rs`: exports it.
- `src/domain/channel/channel.rs`: `reconcile_interval` field, `create` param, `with_reconcile_interval`.
- `src/domain/channel/channel_view.rs`: `reconcile_interval` field.
- `src/domain/playlist/playlist.rs`: `reconcile_interval` field, `create` param, `with_reconcile_interval`.
- `src/domain/task/scheduled_task.rs`: `bring_forward` transition.
- `src/infrastructure/repositories/sqlite_task_repository.rs`: `find_pending` on `TaskRepository`.
- `src/infrastructure/repositories/sqlite_channel_repository.rs`, `sqlite_playlist_repository.rs`: read/write the new column.
- `src/domain/services/channel_creator.rs`, `playlist_creator.rs`: take the interval.
- `src/domain/services/channel_updater.rs`: interval in `update_settings`; brings the pending reconcile forward.
- `src/domain/services/playlist_updater.rs`: `update_exclude_from_home` replaced by `update_settings`; brings the pending reconcile forward.
- `src/domain/services/channel_view_searcher.rs`: maps the interval into `ChannelView`.
- `src/domain/services/channel_video_reconciler.rs`, `playlist_video_reconciler.rs`: drop `reconcile_interval_seconds`; schedule from the entity re-read at the end of the pass.
- `src/application/http/channels/dto.rs`, `mod.rs`: interval on create/update requests and both responses; `NOTHING_TO_UPDATE` becomes "Request must change the quality, the video limit or the reconcile interval".
- `src/application/http/playlists/dto.rs`, `mod.rs`: interval on create/update requests and response; `exclude_from_home` optional on update; new `NOTHING_TO_UPDATE` "Request must change exclude from home or the reconcile interval".
- `src/serve.rs`: removes `reconcile_interval_seconds()` / `DEFAULT_RECONCILE_INTERVAL_SECONDS`; passes task repository + clock to the updaters.
- `doc/INSTALLATION.md`, `doc/ARCHITECTURE.md`, `.env.example`: drop the env var, describe the per-source setting.

Web:
- `web/src/api/types.ts`, `web/src/test/helpers.tsx`: `reconcile_interval_minutes` on `ChannelListItem`, `PlaylistListItem` and builders.
- `web/src/api/client.ts`: interval on create requests; `UpdatePlaylistRequest` (both fields optional).
- `web/src/api/queries.ts`: `useUpdatePlaylistSettings`.
- `web/src/lib/reconcileInterval.ts` (new): minutes ↔ `{ amount, unit }` conversion and validation.
- `web/src/components/ReconcileIntervalField.tsx` (new): number input + unit `Select`, shared by four dialogs.
- `web/src/components/EditChannelDialog.tsx`, `web/src/lib/channelSettings.ts`: interval field and change detection.
- `web/src/components/EditPlaylistDialog.tsx` (new): interval-only dialog.
- `web/src/components/AddChannelDialog.tsx`, `AddPlaylistDialog.tsx`: field under "Advanced options".
- `web/src/components/Sidebar.tsx`: playlist rows pass `onEditRequest`; owns an `EditPlaylistDialog`.
- `web/src/components/PlaylistDetail.tsx`: header `onEditRequest`, renders `EditPlaylistDialog`.
- `smoke-tests/tests/playlist.spec.js`: edit playlist interval flow (exercises the PATCH route).

## Types & Signatures

```rust
// domain/shared/reconcile_interval.rs
pub const DEFAULT_RECONCILE_INTERVAL_MINUTES: u32 = 180;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileInterval(u32); // minutes, 1..=1440 (24 hours)
impl ReconcileInterval {
    pub fn new(minutes: i64) -> Result<Self, ValidationError>; // "Reconcile interval must be at least 1 minute (got {minutes})" / "…must be at most 1440 minutes (got {minutes})"
    pub fn default_interval() -> Self;                          // 180
    pub fn minutes(&self) -> u32;
    pub fn duration(&self) -> chrono::Duration;
}

// domain/channel/channel.rs  (Channel, ChannelView gain `pub reconcile_interval: ReconcileInterval`)
impl Channel {
    pub fn create(id, name, youtube_channel_id, quality, video_limit,
                  reconcile_interval: ReconcileInterval, path, avatar_filename, created_at) -> Self;
    pub fn with_reconcile_interval(self, reconcile_interval: ReconcileInterval) -> Self;
}

// domain/playlist/playlist.rs  (Playlist gains `pub reconcile_interval: ReconcileInterval`)
impl Playlist {
    pub fn create(id, name, path, quality, kind, exclude_from_home,
                  reconcile_interval: ReconcileInterval, created_at) -> Self;
    pub fn with_reconcile_interval(self, reconcile_interval: ReconcileInterval) -> Self;
}

// domain/task/scheduled_task.rs
impl ScheduledTask {
    /// Runs no later than `run_at`; a task already due earlier keeps its time.
    pub fn bring_forward(self, run_at: DateTime<Utc>, now: DateTime<Utc>) -> Self;
}

// infrastructure/repositories/sqlite_task_repository.rs
pub trait TaskRepository {
    /// The pending task with this type and payload, if any.
    fn find_pending(&self, task: &Task) -> anyhow::Result<Option<ScheduledTask>>;
    // ...existing
}

// domain/services/channel_creator.rs
fn create(&self, id: ChannelHandle, quality: Quality, video_limit: VideoLimit,
          reconcile_interval: ReconcileInterval, path: PlaylistPath)
    -> Result<CreateChannelOutcome, CreateChannelError>;

// domain/services/playlist_creator.rs
fn create(&self, id: PlaylistId, path: PlaylistPath, quality: Quality, exclude_from_home: bool,
          reconcile_interval: ReconcileInterval)
    -> Result<CreatePlaylistOutcome, CreatePlaylistError>;

// domain/services/channel_updater.rs
impl ChannelUpdater {
    pub fn new(repository: Arc<dyn ChannelRepository>, task_repository: Arc<dyn TaskRepository>,
               clock: Arc<dyn Clock>) -> Self;
}
fn update_settings(&self, id: ChannelHandle, quality: Option<Quality>, video_limit: Option<VideoLimit>,
                   reconcile_interval: Option<ReconcileInterval>) -> Result<Channel, UpdateChannelError>;

// domain/services/playlist_updater.rs
impl PlaylistUpdater {
    pub fn new(repository: Arc<dyn PlaylistRepository>, task_repository: Arc<dyn TaskRepository>,
               clock: Arc<dyn Clock>) -> Self;
}
fn update_settings(&self, id: PlaylistId, exclude_from_home: Option<bool>,
                   reconcile_interval: Option<ReconcileInterval>) -> Result<Playlist, UpdatePlaylistError>;

// reconcilers: `new(..)` loses `reconcile_interval_seconds: i64`
fn schedule_next_reconcile(&self, id: &ChannelHandle) -> anyhow::Result<()>; // re-finds; None → no schedule
fn schedule_next_reconcile(&self, id: &PlaylistId) -> anyhow::Result<()>;

// application/http/channels/dto.rs
pub struct CreateChannelRequest { /* … */ #[serde(default)] pub reconcile_interval_minutes: Option<i64> }
pub struct UpdateChannelRequest { /* … */ #[serde(default)] pub reconcile_interval_minutes: Option<i64> }
pub struct ChannelResponse         { /* … */ pub reconcile_interval_minutes: u32 }
pub struct ChannelListItemResponse { /* … */ pub reconcile_interval_minutes: u32 }

// application/http/playlists/dto.rs
pub struct CreatePlaylistRequest { /* … */ #[serde(default)] pub reconcile_interval_minutes: Option<i64> }
pub struct UpdatePlaylistRequest {
    #[serde(default)] pub exclude_from_home: Option<bool>,
    #[serde(default)] pub reconcile_interval_minutes: Option<i64>,
}
pub struct PlaylistResponse { /* … */ pub reconcile_interval_minutes: u32 }
```

```ts
// lib/reconcileInterval.ts
export type IntervalUnit = 'minutes' | 'hours'
export interface IntervalForm { amount: string; unit: IntervalUnit }
export const DEFAULT_INTERVAL_FORM: IntervalForm // { amount: '3', unit: 'hours' }
export function intervalForm(minutes: number): IntervalForm   // hours when divisible by 60
export function intervalMinutes(form: IntervalForm): number | null // null when invalid

// components/ReconcileIntervalField.tsx
export function ReconcileIntervalField(props: {
  id: string; value: IntervalForm; onChange: (value: IntervalForm) => void
}): JSX.Element

// api/client.ts
export interface UpdatePlaylistRequest { exclude_from_home?: boolean; reconcile_interval_minutes?: number }
export function updatePlaylist(id: string, body: UpdatePlaylistRequest): Promise<PlaylistListItem>
// CreateChannelRequest / CreatePlaylistRequest / UpdateChannelRequest gain reconcile_interval_minutes

// api/queries.ts
export function useUpdatePlaylistSettings(): (id: string, changes: UpdatePlaylistRequest) => Promise<void>

// components/EditPlaylistDialog.tsx
export function EditPlaylistDialog(props: {
  playlist: PlaylistListItem; open: boolean; onOpenChange: (open: boolean) => void
  onSave: (changes: UpdatePlaylistRequest) => Promise<void>
}): JSX.Element
```

## Call Stack

Recurring reconcile (task):
1. `ReconcileChannelTask::run(payload)` → `ChannelVideoReconciler::reconcile(id)`
2. → `find_channel(&id)` → `reconcile_channel(&channel, SkipMembership)`
3. → `schedule_next_reconcile(&id)` → `channel_repository.find(&id)`; `Some(channel)` → `task_repository.schedule(&Task::ReconcileChannel { channel_id }, clock.now() + channel.reconcile_interval.duration())`; `None` → return `Ok(())`
4. Playlists: same through `PlaylistVideoReconciler`. `initial_reconcile` uses the same `schedule_next_reconcile`.

Update channel:
1. `PATCH /api/channels/{handle}` → `update_channel(State<ChannelUpdater>, Path, Json<UpdateChannelRequest>)`
2. → `ReconcileInterval::new` via `.map(..).transpose()?`; all three `None` → 400 `NOTHING_TO_UPDATE`
3. → `run_blocking(update_settings(id, quality, video_limit, reconcile_interval))`
4. → `find_channel(&id)?` → `apply_settings(channel, quality, video_limit, reconcile_interval)` → `repository.update(&channel)?`
5. → if `reconcile_interval` is `Some(i)`: `bring_forward_next_reconcile(&Task::ReconcileChannel { channel_id }, i)` → `task_repository.find_pending(&task)?`; `Some(t)` → `task_repository.update(&t.bring_forward(now + i.duration(), now))`

Update playlist: same shape via `update_playlist` → `PlaylistUpdater::update_settings(id, exclude_from_home, reconcile_interval)` → `Task::ReconcilePlaylist { playlist_id }`.

Create: `create_channel` / `create_playlist` → `request.reconcile_interval_minutes.map(ReconcileInterval::new).transpose()?.unwrap_or_else(ReconcileInterval::default_interval)` → `creator.create(.., reconcile_interval, ..)` → `Channel::create(..)` / `Playlist::create(..)`.

Web edit playlist:
1. `Sidebar` playlist row / `PlaylistDetail` header → `EntryActionsMenu` "Edit settings" → `onEditRequest()` → editing playlist id state
2. → `<EditPlaylistDialog playlist onSave />` → `intervalMinutes(form)`; unchanged → close; changed → `onSave({ reconcile_interval_minutes })`
3. → `useUpdatePlaylistSettings()(id, changes)` → `updatePlaylist(id, changes)` → `PATCH /api/playlists/{id}` → invalidate library

Web create: `AddChannelDialog` / `AddPlaylistDialog` submit → `intervalMinutes(form.interval)` → `createChannel({ …, reconcile_interval_minutes })` / `createPlaylist({ … })`.

## Test Plan

Behaviour (Rust acceptance tests, through the adapters):

Channel tasks (`application/tasks/reconcile_channel_task.rs`):
1. `it_should_always_schedule_the_next_reconcile` (updated): next reconcile at `now + 180 min` for a channel stored with the default interval.
2. `it_should_schedule_the_next_reconcile_after_the_channels_interval`: channel with 30 min → pending task at `now + 30 min`.
3. `it_should_not_schedule_the_next_reconcile_if_channel_deleted_during_the_pass`: channel removed from the repository by the fake listing → no pending reconcile task.

Playlist tasks (`application/tasks/reconcile_playlist_task.rs`):
4. `it_should_always_schedule_the_next_reconcile` (updated): `now + 180 min`.
5. `it_should_schedule_the_next_reconcile_after_the_playlists_interval`: 720 min → `now + 720 min`.
6. `it_should_not_schedule_the_next_reconcile_if_playlist_deleted_during_the_pass`.

Channel HTTP (`application/http/channels/mod.rs`):
7. `it_should_create_a_channel_with_the_default_interval`: response and stored channel have 180.
8. `it_should_create_a_channel_with_its_interval`: request 30 → response/stored 30.
9. `it_should_fail_to_create_if_invalid_interval_provided`: 0 → 400 exact message (unmigrated DB).
10. `it_should_keep_the_interval_of_an_existing_channel_on_duplicate_create`.
11. `it_should_update_a_channels_interval`: response + stored channel with 60, other fields unchanged.
12. `it_should_bring_the_next_reconcile_forward_if_interval_shortened`: pending reconcile at `now + 20h`, update to 60 → pending task `run_at = now + 60 min`, `updated_at = now`; `it_should_stamp_the_brought_forward_reconcile_with_the_update_time` schedules it under an earlier clock so `updated_at` differs from `created_at` (same pair for playlists, item 22).
13. `it_should_keep_the_next_reconcile_if_interval_lengthened`: pending at `now + 30 min`, update to 1440 → task unchanged.
14. `it_should_fail_to_update_if_invalid_interval_provided`: 0 → 400.
15. `it_should_fail_to_update_if_nothing_to_update` (updated): request with no field, including no interval.
16. `it_should_list_channels_with_their_interval`.

Playlist HTTP (`application/http/playlists/mod.rs`):
17. `it_should_create_a_playlist_with_the_default_interval`.
18. `it_should_create_a_playlist_with_its_interval`.
19. `it_should_fail_to_create_if_invalid_interval_provided`.
20. `it_should_keep_the_interval_of_an_existing_playlist_on_duplicate_create`.
21. `it_should_update_a_playlists_interval`.
22. `it_should_bring_the_next_reconcile_forward_if_interval_shortened`.
23. `it_should_keep_the_next_reconcile_if_interval_lengthened`.
24. `it_should_fail_to_update_if_nothing_to_update` (replaces `it_should_fail_to_update_if_exclude_from_home_missing`).
25. `it_should_fail_to_update_if_invalid_interval_provided`.
26. `it_should_list_playlists_with_their_interval`.

Value object (`domain/shared/reconcile_interval.rs`):
27. `it_should_accept_one_minute`, `it_should_reject_zero`, `it_should_reject_a_negative_interval`, `it_should_accept_a_day`, `it_should_reject_more_than_a_day` (one cycle each).

Web (Vitest, `mockApi`):
28. `lib/reconcileInterval`: `intervalForm(180)` → 3 hours; `intervalForm(90)` → 90 minutes; `intervalMinutes({6,'hours'})` → 360; `intervalMinutes` → `null` for `''`, `'0'`, `'1.5'` and above 24 hours (`25` hours, `1441` minutes).
29. `EditChannelDialog`: prefilled with "3 hours"; changing only the interval sends `{ reconcile_interval_minutes: 30 }` and no reconcile.
30. `EditChannelDialog`: invalid interval flags the field and sends nothing.
31. `EditPlaylistDialog`: opens titled "Edit Lofi beats settings" prefilled with 3 hours.
32. `EditPlaylistDialog`: saving 30 minutes sends PATCH `{ reconcile_interval_minutes: 30 }`, no reconcile, dialog closes.
33. `EditPlaylistDialog`: saving unchanged sends nothing; save failure keeps the dialog open with the error; more than 24 hours flags the field and sends nothing.
34. `Sidebar`: a playlist row menu lists Sync, Exclude from home, Edit settings, Delete; choosing Edit settings opens the dialog.
35. `PlaylistDetail`: header menu offers Edit settings and opens the dialog.
36. `AddChannelDialog`: submits `reconcile_interval_minutes: 180` by default and 30 when set to 30 minutes.
37. `AddPlaylistDialog`: same as 36.

Infrastructure:

`sqlite_task_repository.rs`:
38. `it_should_find_the_pending_task_with_the_same_type_and_payload`.
39. `it_should_not_find_a_running_or_completed_task_as_pending`.
40. `it_should_not_find_a_pending_task_of_another_source`: same type, different payload → `None`.

`sqlite_channel_repository.rs` / `sqlite_playlist_repository.rs`:
41. `it_should_round_trip_the_channels_reconcile_interval`.
42. `it_should_round_trip_the_playlists_reconcile_interval`.

`sqlite_migrations.rs`:
43. `it_should_give_existing_channels_and_playlists_the_default_interval`: rows inserted before 0009 read back with 180.
