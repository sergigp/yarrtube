## Files

- `src/domain/services/playlist_video_reconciler.rs`: new private `list_current_videos` turns a failed listing into a logged `None`, and `sync_membership_with_youtube` returns empty `MembershipChanges` for it. This mirrors `ChannelVideoReconciler`.
- `src/infrastructure/repositories/youtube_playlist_items_repository.rs`: `FakeYoutubePlaylistItemsRepository` gets a `fails` flag plus `with_videos` / `failing` constructors, the same shape as `FakeChannelVideosRepository`.
- `src/application/tasks/reconcile_playlist_task.rs`: primary-adapter acceptance test. The existing fake struct literals move to `with_videos`.
- `src/application/http/playlists/mod.rs`: on-demand acceptance test, since the response changes from an internal error to success. The existing fake struct literals move to `with_videos`.

## Types & Signatures

```rust
// playlist_video_reconciler.rs (private, block 4)
fn list_current_videos(&self, playlist: &Playlist) -> Option<Vec<YoutubePlaylistItem>>;

// youtube_playlist_items_repository.rs (#[cfg(test)])
#[derive(Default)]
pub struct FakeYoutubePlaylistItemsRepository {
    pub(crate) videos: std::sync::Mutex<Vec<YoutubePlaylistItem>>,
    fails: bool,
}

impl FakeYoutubePlaylistItemsRepository {
    pub fn with_videos(videos: Vec<YoutubePlaylistItem>) -> Self;
    pub fn failing() -> Self;
}
```

## Call Stack

Recurring pass, listing fails:
1. `ReconcilePlaylistTask::handle(payload)`
2. → `PlaylistVideoReconciler::reconcile(id: PlaylistId)`
3. → `find_playlist(&id)` → `reconcile_playlist(&playlist)`
4. → `sync_membership_with_youtube(&playlist)`
5. → `list_current_videos(&playlist)` → `YoutubePlaylistItemsRepository::list_current_videos(&playlist.id)` returns `Err`, which is logged (`warn`, with the cause chain) and becomes `None`
6. → `MembershipChanges::default()`, then every local step runs unchanged (`take_local_snapshot`, ..., `delete_orphaned_files`)
7. → `schedule_next_reconcile(&id)`

On-demand: `force_reconcile(id)` follows steps 3–6 and skips step 7. The handler responds `204 NO_CONTENT`.

## Test Plan

1. Behaviour tests
   - `reconcile_playlist_task::it_should_keep_videos_if_listing_fails`: a stored pending video and a failing listing. Asserts `Ok(())`, videos and playlist videos unchanged, no events, and non-completed tasks equal `[download_video, fetch_thumbnail, next_reconcile]`. The download and thumbnail tasks prove the local steps ran; `next_reconcile` proves the playlist is rescheduled.
   - `http::playlists::it_should_keep_videos_on_reconcile_if_listing_fails`: the same seed through the on-demand handler. Asserts `Ok(StatusCode::NO_CONTENT)`, videos unchanged, no events, and no `ReconcilePlaylist` task scheduled.
2. Infrastructure tests
   - None. Only the test fake changes; the real adapter's error behaviour is unchanged.
