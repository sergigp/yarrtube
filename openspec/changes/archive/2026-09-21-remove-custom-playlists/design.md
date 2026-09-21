## Files

- `src/domain/services/custom_playlist_video_adder.rs` — deleted, custom-playlist add-video use case removed
- `src/domain/services/custom_playlist_video_remover.rs` — deleted, custom-playlist remove-video use case removed
- `src/domain/services/mod.rs` — drops the two `mod`/`pub use` lines for the deleted services
- `src/domain/services/playlist_creator.rs` — drops `create_custom`, `CreatePlaylistOutcome` and `create` untouched
- `src/domain/services/video_reconciler.rs` — `run_reconcile_pass` always syncs membership; drops the `PlaylistKind` check and import
- `src/domain/playlist/playlist_kind.rs` — drops the `Custom` variant and its parse/display arms
- `src/domain/playlist/errors.rs` — deletes `CreateCustomPlaylistError`
- `src/domain/playlist/mod.rs` — drops the `CreateCustomPlaylistError` export
- `src/domain/video/errors.rs` — deletes `AddVideoToCustomPlaylistError` and `RemoveVideoFromPlaylistError` (both are used only by the deleted services/handlers)
- `src/domain/video/mod.rs` — drops both error exports
- `src/application/http/custom_playlists/mod.rs`, `dto.rs` — deleted, the whole module
- `src/application/http/mod.rs` — drops the `custom_playlists` module, its 3 routes, and the two `AppState` fields
- `src/serve.rs` — drops construction of `custom_playlist_video_adder`/`_remover` and their `AppState` entries
- `web/src/api.js` — deletes `deleteVideoFromCustomPlaylist`
- `web/src/components/PlaylistDetail.jsx` — deletes the `canDelete`/`kind === 'custom'` gate and the Delete button + `ConfirmDialog` it guarded
- `README.md` — deletes the two "Coming soon: custom playlists..." lines
- `openspec/specs/custom-playlist-crud/spec.md` — removed (via this change's delta, at archive time)
- `openspec/specs/playlist-crud/spec.md` — trimmed (via this change's delta, at archive time)

## Types & Signatures

```rust
// playlist_kind.rs
pub enum PlaylistKind {
    YoutubeLinked,
}
// new(), as_str(), Display all drop the "custom" arm; new() now only accepts "youtube_linked"

// video_reconciler.rs
fn run_reconcile_pass(&self, playlist: &Playlist) -> anyhow::Result<()> {
    // always calls self.sync_playlist_membership(playlist), no kind check
}

// http/mod.rs
pub struct AppState {
    pub playlist_creator: PlaylistCreator,
    pub playlist_deleter: PlaylistDeleter,
    pub playlist_searcher: PlaylistSearcher,
    pub video_reconciler: VideoReconciler,
    pub video_searcher: VideoSearcher,
    pub task_view_searcher: TaskViewSearcher,
    pub channel_service: ChannelService,
    pub channel_video_reconciler: ChannelVideoReconciler,
}
```

Deleted entirely (no replacement): `CustomPlaylistVideoAdder`, `CustomPlaylistVideoRemover`,
`PlaylistCreator::create_custom`, `CreateCustomPlaylistError`,
`AddVideoToCustomPlaylistError`, `RemoveVideoFromPlaylistError`,
`custom_playlists::{create_custom_playlist, add_video, remove_video}`.

## Call Stack

Reconcile, before -> after:

```
run_reconcile_pass(playlist)
  if playlist.kind == YoutubeLinked:      run_reconcile_pass(playlist)
    sync_playlist_membership(playlist)  ->  sync_playlist_membership(playlist)
  reconcile_filesystem(playlist)            reconcile_filesystem(playlist)
```

`ReconcileOnPlaylistCreated::handle` is unchanged (it already just calls
`video_reconciler.reconcile(playlist_id)` regardless of kind).

## Test Plan

Deleted with their source files:
- all tests in `custom_playlist_video_adder.rs`, `custom_playlist_video_remover.rs`,
  `application/http/custom_playlists/mod.rs`

Updated:
- `playlist_kind.rs`: `it_should_parse_each_valid_kind_value` drops the
  `"custom"` case; add `it_should_reject_the_removed_custom_kind_value` (or
  fold into the existing invalid-value test) asserting `PlaylistKind::new("custom")` errors
- `playlist_creator.rs` tests: drop `it_should_be_true_when_the_colliding_playlist_is_of_the_other_kind`
  (no other kind exists); remaining `path_used_by_another_playlist` tests keep using `PlaylistKind::YoutubeLinked`
- `video_reconciler.rs` tests: any case seeding a `PlaylistKind::Custom` playlist to assert
  a no-op reconcile is deleted; YouTube-linked reconcile tests are unaffected
- `reconcile_on_playlist_created.rs`: delete
  `it_should_run_a_harmless_no_op_reconcile_pass_for_a_newly_created_custom_playlist`
- `sqlite_playlist_repository.rs` tests: any using `PlaylistKind::Custom` switch to `PlaylistKind::YoutubeLinked`

No new tests — this change only removes behavior.
