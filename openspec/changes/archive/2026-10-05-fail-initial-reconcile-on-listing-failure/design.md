## Files

- `src/domain/services/playlist_video_reconciler.rs`: new `initial_reconcile` on the `…Api` trait. A private `ListingFailure` enum picks per pass whether a failed listing skips membership or fails the pass.
- `src/domain/services/channel_video_reconciler.rs`: the same for channels.
- `src/application/subscribers/reconcile_on_playlist_created.rs`: calls `initial_reconcile`, plus the listing-failure acceptance test.
- `src/application/subscribers/reconcile_on_channel_created.rs`: calls `initial_reconcile`, plus the listing-failure acceptance test.

## Types & Signatures

```rust
// playlist_video_reconciler.rs (block 2)
pub trait PlaylistVideoReconcilerApi: Send + Sync {
    fn reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;
    fn initial_reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;
    fn force_reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;
}

// playlist_video_reconciler.rs (block 4 + bottom)
fn reconcile_playlist(&self, playlist: &Playlist, on_listing_failure: ListingFailure) -> anyhow::Result<()>;
fn sync_membership_with_youtube(&self, playlist: &Playlist, on_listing_failure: ListingFailure) -> anyhow::Result<MembershipDelta>;
fn list_current_videos(&self, playlist: &Playlist, on_listing_failure: ListingFailure) -> anyhow::Result<Option<Vec<YoutubePlaylistItem>>>;

#[derive(Clone, Copy)]
enum ListingFailure { SkipMembership, FailPass }

// channel_video_reconciler.rs: same shape over ChannelHandle / Channel / ChannelVideoListing.
```

The enum is private per reconciler, not shared: each file's listing failure has its own error context ("failed to list playlist items" / "failed to list channel videos").

## Call Stack

Creation, listing fails:
1. `ReconcileOnPlaylistCreated::handle(payload)`
2. → `PlaylistVideoReconciler::initial_reconcile(id: PlaylistId)`
3. → `find_playlist(&id)` → `reconcile_playlist(&playlist, ListingFailure::FailPass)`
4. → `sync_membership_with_youtube(&playlist, FailPass)` → `list_current_videos(&playlist, FailPass)`
5. → `YoutubePlaylistItemsRepository::list_current_videos(&playlist.id)` returns `Err`, which becomes `Err(e.context("failed to list playlist items"))` and propagates. `schedule_next_reconcile` never runs.
6. → the events consumer records the failure and retries the `PlaylistCreated` event with backoff.

Creation, listing succeeds: same as `reconcile`, so steps 3–4 run the full pass, then `schedule_next_reconcile(&id)`.

`reconcile` and `force_reconcile` pass `ListingFailure::SkipMembership`, so their behaviour is unchanged.

Channels: `ReconcileOnChannelCreated::handle` → `ChannelVideoReconciler::initial_reconcile(id: ChannelHandle)`, with the same steps. The error context is "failed to list channel videos".

## Test Plan

1. Behaviour tests
   - `reconcile_on_playlist_created::it_should_fail_if_listing_fails`: a stored playlist with a failing listing. Asserts `Err("failed to list playlist items")`, no videos, no tasks, no events.
   - `reconcile_on_channel_created::it_should_fail_if_listing_fails`: a stored channel with a failing discovery. Asserts `Err("failed to list channel videos")`, no videos, no tasks, no events.
2. Infrastructure tests
   - None. No adapter changes.
