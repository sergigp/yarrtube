## 1. Backend: on-demand reconcile endpoint

- [x] 1.1 Add `reconcile_playlist` HTTP handler in `src/http/playlists/mod.rs`: parses the `id` path param into `PlaylistId` (400 on invalid), calls `state.video_service.reconcile_playlist(id)` via `tokio::task::spawn_blocking` (mirroring `create_playlist`'s pattern), returns `204 No Content` on `Ok(())`, and `500 Internal Server Error` with the error's message on `Err`.
- [x] 1.2 Register `POST /playlists/{id}/reconcile` in `src/http/mod.rs`'s `api_router`, routed to the new handler.
- [x] 1.3 Add handler tests in `src/http/playlists/mod.rs` covering: successful reconcile of a YouTube-linked playlist (asserts 204 and that membership/filesystem changes from a fake YouTube/video-file repository are applied, matching the assertions style already used for `reconcile_playlist` in `src/tasks/reconcile_playlist_task.rs`), successful reconcile of a custom playlist (204, no YouTube call), reconcile of a nonexistent playlist ID (still 204, no state changes), and invalid playlist ID (400).
- [x] 1.4 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked` and confirm all pass.

## 2. Frontend: reconcile action

- [x] 2.1 Add `reconcilePlaylist(id)` to `web/src/api.js`, mirroring `deletePlaylist`'s `POST` fetch/error-handling pattern against the new endpoint.
- [x] 2.2 Add a "Reconcile" `DropdownMenu.Item` to `web/src/components/PlaylistActionsMenu.jsx`, calling `reconcilePlaylist(playlist.id)` directly on select (no confirmation dialog, since the action is non-destructive), and surface a failure (e.g. a simple inline/alert message) if the call rejects.
- [x] 2.3 Manually verify in the browser: create a YouTube-linked playlist, add a video to the underlying YouTube playlist, click "Reconcile" from the actions menu, and confirm the new video appears without waiting for the hourly interval.
