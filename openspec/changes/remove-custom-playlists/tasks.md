## 1. Domain layer

- [x] 1.1 Drop the `Custom` variant from `PlaylistKind` (`src/domain/playlist/playlist_kind.rs`): remove its parse/`as_str`/`Display` arms; update `it_should_parse_each_valid_kind_value` to drop the `"custom"` case and confirm `PlaylistKind::new("custom")` now returns the existing invalid-value error
- [x] 1.2 Delete `src/domain/services/custom_playlist_video_adder.rs` and `src/domain/services/custom_playlist_video_remover.rs`, and their `mod`/`pub use` lines in `src/domain/services/mod.rs`
- [x] 1.3 Remove `PlaylistCreator::create_custom` from `src/domain/services/playlist_creator.rs`; drop the `it_should_be_true_when_the_colliding_playlist_is_of_the_other_kind` test and switch remaining `path_used_by_another_playlist` tests to `PlaylistKind::YoutubeLinked`; verify with `cargo test playlist_creator`
- [x] 1.4 Simplify `VideoReconciler::run_reconcile_pass` (`src/domain/services/video_reconciler.rs`) to always call `sync_playlist_membership`, dropping the `PlaylistKind` check and its now-unused import; delete any test seeding a `Custom` playlist to assert a no-op reconcile; verify with `cargo test video_reconciler`
- [x] 1.5 Delete `CreateCustomPlaylistError` from `src/domain/playlist/errors.rs` and its export in `src/domain/playlist/mod.rs`
- [x] 1.6 Delete `AddVideoToCustomPlaylistError` and `RemoveVideoFromPlaylistError` from `src/domain/video/errors.rs` and their exports in `src/domain/video/mod.rs`
- [x] 1.7 In `reconcile_on_playlist_created.rs`, delete `it_should_run_a_harmless_no_op_reconcile_pass_for_a_newly_created_custom_playlist`; update its doc comment referencing `Custom` playlists
- [x] 1.8 Update any remaining test in `sqlite_playlist_repository.rs` that constructs `PlaylistKind::Custom` to use `PlaylistKind::YoutubeLinked`; verify with `cargo test sqlite_playlist_repository`

## 2. HTTP layer and wiring

- [x] 2.1 Delete `src/application/http/custom_playlists/` (`mod.rs`, `dto.rs`)
- [x] 2.2 In `src/application/http/mod.rs`, remove the `custom_playlists` module declaration, the three `/custom-playlists*` routes, the `custom_playlist_video_adder`/`custom_playlist_video_remover` imports, and the two corresponding `AppState` fields
- [x] 2.3 In `src/serve.rs`, remove construction of `custom_playlist_video_adder` and `custom_playlist_video_remover` and their entries in the `AppState` literal
- [x] 2.4 Verify with `cargo build --release` that the binary compiles with no dangling references to the deleted types

## 3. Web UI

- [x] 3.1 Remove `deleteVideoFromCustomPlaylist` from `web/src/api.js`
- [x] 3.2 In `web/src/components/PlaylistDetail.jsx`, remove the `canDelete`/`kind === 'custom'` check, the Delete button, and the `ConfirmDialog` it guarded (and the now-unused import of `deleteVideoFromCustomPlaylist`); verify the playlist detail view still renders a video without a Delete button

## 4. Docs

- [x] 4.1 Remove the two "Coming soon: custom playlists..." lines from `README.md`

## 5. Verification

- [ ] 5.1 Run `cargo fmt --all -- --check`
- [ ] 5.2 Run `cargo clippy --all-targets --all-features --locked -- -D warnings`
- [ ] 5.3 Run `cargo test --locked` and confirm the full suite passes with no lingering references to removed custom-playlist types
- [ ] 5.4 Run `scripts/run-local.sh`, confirm the server starts, `GET /playlists` still returns `kind: "youtube_linked"` for existing playlists, and `POST /custom-playlists` now 404s (route no longer exists)
