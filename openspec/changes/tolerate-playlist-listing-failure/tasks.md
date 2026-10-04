## 1. Walking skeleton

- [ ] 1.1 Add the `fails` flag plus the `with_videos` / `failing` constructors to `FakeYoutubePlaylistItemsRepository`. `list_current_videos` keeps ignoring `fails` for now. Move every existing struct literal of the fake to `with_videos`. The private `PlaylistVideoReconciler::list_current_videos` is added in 2.1, because any body for it already changes behaviour. Verify: `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [ ] 2.1 `reconcile_playlist_task::it_should_keep_videos_if_listing_fails`: a failed listing leaves stored videos and membership untouched, publishes no events, still runs the local steps and still schedules the next reconcile. Make the fake honour `fails`, then add `list_current_videos` (log and return `None`) and the early `MembershipChanges::default()` return in `sync_membership_with_youtube`. Verify: the test passes.
- [ ] 2.2 `http::playlists::it_should_keep_videos_on_reconcile_if_listing_fails`: an on-demand reconcile with a failed listing responds `204 NO_CONTENT`, leaves videos untouched and schedules no `ReconcilePlaylist` task. Verify: the test passes.

## 3. Infrastructure adapters (TDD)

- [ ] 3.1 None: the real `YoutubeApiPlaylistItemsRepository` is unchanged.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 Sync the delta into `openspec/specs/playlist-reconciliation/spec.md` at archive time. Verify: `openspec validate tolerate-playlist-listing-failure --strict` passes.
