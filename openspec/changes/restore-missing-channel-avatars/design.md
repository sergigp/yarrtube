## Files

- `src/domain/services/channel_avatar_restorer.rs` (new): domain service that decides whether a channel's avatar is missing and restores it.
- `src/domain/services/mod.rs`: registers and re-exports `ChannelAvatarRestorer`.
- `src/domain/services/channel_video_reconciler.rs`: runs the restorer at the start of each pass, before writing the show files.
- `src/domain/channel/channel.rs`: `with_avatar_filename` state transition.
- `src/infrastructure/repositories/filesystem_channel_avatar_repository.rs`: `exists` on the port, the filesystem adapter and the fake.
- `src/serve.rs`: wires the restorer into `channel_video_reconciler`.
- `src/application/tasks/reconcile_channel_task.rs`, `src/application/subscribers/reconcile_on_channel_created.rs`, `src/application/http/channels/mod.rs`: pass the restorer wherever tests build a `ChannelVideoReconciler`; the behaviour tests live in the task.

## Types & Signatures

```rust
// domain/services/channel_avatar_restorer.rs
pub struct ChannelAvatarRestorer {
    channel_repository: Arc<dyn ChannelRepository>,
    lookup: Arc<dyn YoutubeChannelRepository>,
    avatar_repository: Arc<dyn ChannelAvatarRepository>,
}

impl ChannelAvatarRestorer {
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        lookup: Arc<dyn YoutubeChannelRepository>,
        avatar_repository: Arc<dyn ChannelAvatarRepository>,
    ) -> Self;
}

pub trait ChannelAvatarRestorerApi: Send + Sync {
    /// Best effort, never fails: returns the channel as given, or updated
    /// with its restored avatar filename.
    fn restore_missing(&self, channel: Channel) -> Channel;
}

// domain/channel/channel.rs
impl Channel {
    pub fn with_avatar_filename(self, avatar_filename: Option<String>) -> Self;
}

// infrastructure/repositories/filesystem_channel_avatar_repository.rs
pub trait ChannelAvatarRepository: Send + Sync {
    // ...existing store / delete / path_of
    fn exists(&self, filename: &str) -> anyhow::Result<bool>;
}

// domain/services/channel_video_reconciler.rs
impl ChannelVideoReconciler {
    pub fn new(/* ...existing..., */ show_file_writer: Arc<ShowFileWriter>,
               channel_avatar_restorer: Arc<ChannelAvatarRestorer>,
               clock: Arc<dyn Clock>, reconcile_interval_seconds: i64) -> Self;
    fn reconcile_channel(&self, channel: Channel, on_listing_failure: ListingFailure)
        -> anyhow::Result<()>; // takes the channel by value now
}
```

An `exists` check that errors counts as the avatar being present, so a filesystem error doesn't trigger a YouTube lookup and download on every pass.

## Call Stack

Reconcile pass (`ReconcileChannelTask`, `ChannelCreated` subscriber, force-reconcile HTTP handler):

1. `ChannelVideoReconciler::{reconcile, initial_reconcile, force_reconcile}(id)`
2. → `reconcile_channel(channel, on_listing_failure)`
3. → `ChannelAvatarRestorer::restore_missing(channel)`
   - `avatar_repository.exists(&avatar_filename)`: if true, return the channel as is
   - `lookup.resolve(&channel.id)` → `ResolvedChannel { avatar_url, .. }`
   - `avatar_repository.store(&channel.id, &avatar_url)` → `Some(filename)`
   - if `filename` differs from the recorded one: `channel_repository.find(&channel.id)` (the channel as stored now, so a settings change made during the download isn't overwritten) → `.with_avatar_filename(Some(filename))` → `channel_repository.update(&channel)`
4. → `ShowFileWriter::write_channel_show(&restored_channel)` (copies the restored avatar as `poster.jpg`)
5. → membership sync and `InternalVideoReconciler::reconcile` (unchanged)

## Test Plan

1. Behaviour tests (`ReconcileChannelTask`):
   1. `it_should_restore_a_missing_channel_avatar`: a recorded `@somechannel.jpg` absent from the avatar store, YouTube returns an avatar URL → the avatar store holds `@somechannel.jpg` from that URL, `poster.jpg` is copied from it, and the stored channel is unchanged.
   2. `it_should_record_a_restored_avatar_for_a_channel_without_one`: a channel with no recorded avatar → the stored channel records `@somechannel.jpg`, and `poster.jpg` is copied from it.
   3. `it_should_keep_a_stored_channel_avatar`: the recorded avatar file is present, YouTube returns a different URL → the avatar store still holds the original file from the original URL.
   4. `it_should_keep_settings_changed_while_restoring_a_channel_avatar`: the channel's video limit is changed while its avatar is being looked up → the stored channel keeps the new limit and records the avatar.
   5. `it_should_sync_the_channel_even_if_its_avatar_cannot_be_restored`: avatar download unavailable → the stored channel is unchanged, the listed video is stored, and the next reconcile is scheduled.
2. Infrastructure tests (`FilesystemChannelAvatarRepository`):
   1. `it_should_report_a_stored_avatar_file_as_existing`: a file written in the avatars directory → `exists` returns `true`.
   2. `it_should_report_an_avatar_as_missing_if_its_directory_is_gone`: the avatars directory doesn't exist → `exists` returns `false`.
