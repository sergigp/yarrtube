## 1. Playlist path uniqueness

- [x] 1.1 Add a way to check whether a path is already used by a different playlist (e.g. a helper over `PlaylistRepository::list()`), and a new `PlaylistError`/`CreatePlaylistError`/`CreateCustomPlaylistError` variant for the rejection; verify with a unit test on the check itself.
- [x] 1.2 Wire the check into `PlaylistService::create_playlist`, after the existing same-ID idempotency lookup and alongside the other structural validations, before any YouTube lookup; verify with unit tests covering: rejected when another playlist already has the path, allowed when re-submitting a playlist's own existing path (idempotent case), allowed when the path is free.
- [x] 1.3 Wire the same check into `PlaylistService::create_custom_playlist`; verify with unit tests covering the same cases, including rejection when the colliding playlist is of the other kind.
- [x] 1.4 Map the new error variant to `400 Bad Request` with a meaningful message in `src/http/playlists/mod.rs` and the custom-playlists HTTP handler; verify with an HTTP-level test asserting `400` for a colliding path on both create endpoints.

## 2. Synchronous video-record cleanup on playlist deletion

- [x] 2.1 Add `VideoRepository::delete_all_for_playlist(playlist_id)` (SQLite and fake implementations); verify with a unit test that it removes every row for that playlist and leaves other playlists' rows untouched.
- [x] 2.2 Add a `path` field to `DomainEvent::PlaylistDeleted` (`src/domain/event/domain_event.rs`), updating `event_type()`/`payload()` and existing tests; verify `cargo test domain_event` passes.
- [x] 2.3 Give `PlaylistService` a `VideoRepository` dependency and update `delete_playlist` to: capture the playlist's path, call `delete_all_for_playlist`, delete the playlist row, then publish `PlaylistDeleted { playlist_id, path }`; update `PlaylistService::new` call sites (composition root, tests).
- [x] 2.4 Verify with a unit test that deleting a playlist with stored video records leaves none of them behind immediately after the call returns (no polling/waiting), for both playlist kinds.
- [x] 2.5 Update the existing `it_should_record_a_playlist_deleted_event_on_successful_deletion` HTTP test (and any other `PlaylistDeleted` assertions) for the new payload shape.

## 3. Async playlist directory cleanup

- [x] 3.1 Add a recursive directory-removal capability to `VideoFileRepository` (or a new narrowly-scoped trait) that no-ops if the directory is already missing; verify with unit tests for: directory with files gets removed, already-missing directory is not an error.
- [x] 3.2 Add `Task::DeletePlaylistFiles { playlist_id, path }` to `src/domain/task/task.rs` (`task_type`, `payload`, decode helper) following the existing `Task::DeleteVideoFile` shape; verify with unit tests mirroring the existing task encode/decode tests.
- [x] 3.3 Add a task handler (e.g. in `src/tasks/`) that calls the new recursive-removal capability with `videos_path`/`path`; register it in `src/tasks/mod.rs::registry()` under `"delete_playlist_files"`; verify with a unit test that it removes the directory and no-ops if already gone.
- [x] 3.4 Add a subscriber for the `"playlist_deleted"` event type (e.g. in `src/subscribers/`) that schedules a `DeletePlaylistFiles` task from the event's `playlist_id`/`path`; register it in `src/subscribers/mod.rs::registry()`; verify with a unit test asserting the task gets scheduled with the right payload.
- [x] 3.5 End-to-end verification: an HTTP-level test that deletes a playlist with a downloaded video, drives the event consumer and task executor's `poll_once`/handler directly (matching how existing tests exercise `DeleteVideoFileOnVideoDeleted`/`DeleteVideoFile`), and asserts the playlist's output directory no longer exists on disk.

## 4. SPA

- [x] 4.1 Update the delete-confirmation copy in `web/src/components/PlaylistActionsMenu.jsx` to reflect that videos and files are now cleaned up automatically; verify by re-reading the rendered dialog text (manual check or existing component test if one covers this dialog).

## 5. Final checks

- [x] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; verify all pass.
