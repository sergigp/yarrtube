## Files

- `src/domain/services/channel_deleter.rs`: takes the task repository; removes the channel's pending reconciles before deleting it.
- `src/domain/services/playlist_deleter.rs`: same for playlists.
- `src/domain/services/channel_video_reconciler.rs`: `schedule_next_reconcile` schedules nothing when a reconcile of the channel is already pending.
- `src/domain/services/playlist_video_reconciler.rs`: same for playlists.
- `src/serve.rs`: passes the task repository to both deleters.

No new port method: `TaskRepository::find_pending` and `delete` already exist. Reconciles run in the single-slot Light lane, so the find-then-schedule check cannot race another reconcile.

## Types & Signatures

```rust
// domain/services/channel_deleter.rs
impl ChannelDeleter {
    pub fn new(repository: Arc<dyn ChannelRepository>, video_repository: Arc<dyn VideoRepository>,
               channel_video_repository: Arc<dyn ChannelVideoRepository>,
               avatar_repository: Arc<dyn ChannelAvatarRepository>,
               event_publisher: Arc<dyn EventPublisher>,
               task_repository: Arc<dyn TaskRepository>) -> Self;
    // private
    fn remove_pending_reconciles(&self, id: &ChannelHandle) -> Result<(), DeleteChannelError>;
}

// domain/services/playlist_deleter.rs
impl PlaylistDeleter {
    pub fn new(repository: Arc<dyn PlaylistRepository>, video_repository: Arc<dyn VideoRepository>,
               playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
               event_publisher: Arc<dyn EventPublisher>,
               task_repository: Arc<dyn TaskRepository>) -> Self;
    // private
    fn remove_pending_reconciles(&self, id: &PlaylistId) -> Result<(), DeletePlaylistError>;
}

// reconcilers: signatures unchanged
fn schedule_next_reconcile(&self, id: &ChannelHandle) -> anyhow::Result<()>; // pending exists → Ok(())
fn schedule_next_reconcile(&self, id: &PlaylistId) -> anyhow::Result<()>;
```

## Call Stack

Delete channel:
1. `DELETE /api/channels/{handle}` → `delete_channel` → `ChannelDeleter::delete(id)`
2. → `find_channel(&id)?` → `delete_channel_videos` → `delete_avatar`
3. → `remove_pending_reconciles(&id)`: repeat `task_repository.find_pending(&Task::ReconcileChannel { channel_id })?` → `Some(t)` → `task_repository.delete(t.id)?`, until `None` (a pre-fix database can hold several)
4. → `delete_channel(&id)` → `publish_channel_deleted`

Delete playlist: same via `PlaylistDeleter::delete` → `remove_pending_reconciles(&id)` with `Task::ReconcilePlaylist { playlist_id }`, after `delete_playlist_videos`, before `delete_playlist`.

Schedule next reconcile (recurring and initial pass):
1. `ReconcileChannelTask::run` / `ReconcileOnChannelCreated` → `reconcile` / `initial_reconcile` → `schedule_next_reconcile(&id)`
2. → `channel_repository.find(&id)?`; `None` → `Ok(())`
3. → `task_repository.find_pending(&task)?`; `Some(_)` → `Ok(())` (the running pass is not pending, so it never blocks itself)
4. → `task_repository.schedule(&task, clock.now() + channel.reconcile_interval.duration())`
5. Playlists: same through `PlaylistVideoReconciler`.

## Test Plan

Behaviour (Rust acceptance tests, through the adapters):

Channel HTTP (`application/http/channels/mod.rs`):
1. `it_should_remove_the_pending_reconcile_of_a_deleted_channel`: pending reconcile of the channel and of another channel → after delete only the other channel's remains.
2. `it_should_remove_every_pending_reconcile_of_a_deleted_channel`: two pending reconciles of the channel → none remain.

Playlist HTTP (`application/http/playlists/mod.rs`):
3. `it_should_remove_the_pending_reconcile_of_a_deleted_playlist`: pending reconcile of the playlist and of another playlist → only the other's remains.
4. `it_should_remove_every_pending_reconcile_of_a_deleted_playlist`: two pending → none remain.

Channel task (`application/tasks/reconcile_channel_task.rs`):
5. `it_should_not_schedule_the_next_reconcile_if_one_is_already_pending`: a pending reconcile of the channel at `now + 5 min` → after the pass, the task list is exactly that pending reconcile.

Playlist task (`application/tasks/reconcile_playlist_task.rs`):
6. `it_should_not_schedule_the_next_reconcile_if_one_is_already_pending`: same for a playlist.

Infrastructure: none (no adapter changes).
