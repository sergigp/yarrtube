## 1. Backend: on-demand reconcile endpoint

- [x] 1.1 Add `reconcile_playlist` HTTP handler in `src/http/playlists/mod.rs`: parses the `id` path param into `PlaylistId` (400 on invalid), calls `state.video_service.force_reconcile_playlist(id)` via `tokio::task::spawn_blocking` (mirroring `create_playlist`'s pattern), returns `204 No Content` on `Ok(())`, and `500 Internal Server Error` with the error's message on `Err`.
- [x] 1.2 Register `POST /playlists/{id}/reconcile` in `src/http/mod.rs`'s `api_router`, routed to the new handler.
- [x] 1.3 Add handler tests in `src/http/playlists/mod.rs` covering: successful reconcile of a YouTube-linked playlist (asserts 204 and that membership/filesystem changes from a fake YouTube/video-file repository are applied, matching the assertions style already used for `reconcile_playlist` in `src/tasks/reconcile_playlist_task.rs`), successful reconcile of a custom playlist (204, no YouTube call), reconcile of a nonexistent playlist ID (still 204, no state changes), and invalid playlist ID (400).
- [x] 1.4 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` and confirm all pass.

## 2. Frontend: reconcile action

- [x] 2.1 Add `reconcilePlaylist(id)` to `web/src/api.js`, mirroring `deletePlaylist`'s `POST` fetch/error-handling pattern against the new endpoint.
- [x] 2.2 Add a "Reconcile" `DropdownMenu.Item` to `web/src/components/PlaylistActionsMenu.jsx`, calling `reconcilePlaylist(playlist.id)` directly on select (no confirmation dialog, since the action is non-destructive), and surface a failure (e.g. a simple inline/alert message) if the call rejects.
- [x] 2.3 Manually verify in the browser: create a YouTube-linked playlist, add a video to the underlying YouTube playlist, click "Reconcile" from the actions menu, and confirm the new video appears without waiting for the hourly interval.

## 3. Backend: split reconcile so on-demand never schedules

- [x] 3.1 In `VideoService` (`src/domain/video/service.rs`), extract the membership-diff-plus-filesystem-heal logic out of `reconcile_playlist` into a private `run_reconcile_pass(&self, playlist: &Playlist) -> anyhow::Result<()>` helper.
- [x] 3.2 Add `pub fn force_reconcile_playlist(&self, id: PlaylistId) -> anyhow::Result<()>`: looks up the playlist (no-op if missing, same as `reconcile_playlist`), calls `run_reconcile_pass`, and returns — without scheduling anything. `reconcile_playlist` keeps calling `run_reconcile_pass` too, followed by its existing next-reconcile scheduling, unchanged.
- [x] 3.3 Update the HTTP handler (`src/http/playlists/mod.rs`) to call `force_reconcile_playlist` instead of `reconcile_playlist`.
- [x] 3.4 Update/add tests: reconciling on demand (once or repeatedly) never adds a pending task (assert the task list stays empty, or unchanged when one was already pending before the on-demand call).
- [x] 3.5 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` and confirm all pass.
