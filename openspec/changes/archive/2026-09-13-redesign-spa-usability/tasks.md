## 1. Backend: task context resolution

- [x] 1.1 Add a read-path helper that decodes a `ScheduledTask`'s `payload` into `playlist_id` and, when present, `video_id` for each `Task` variant, and verify with unit tests covering `ReconcilePlaylist`, `DownloadVideo`, and `DeleteVideoFile` payloads.
- [x] 1.2 In `src/http/tasks/mod.rs`'s `list_tasks` handler, collect the distinct `playlist_id`s (and `video_id`s) across the fetched tasks and batch-resolve them via the playlist/video repositories, and verify no N+1 query pattern is introduced (one lookup call per distinct ID set, not per task).
- [x] 1.3 Extend `TaskResponse` (`src/http/tasks/dto.rs`) with optional `playlist_name`, `video_id`, and `video_title` fields populated from the resolution in 1.2, defaulting to absent when a referenced playlist/video can't be found, and verify against the task-listing spec delta scenarios (playlist resolved, video resolved, reference missing).
- [x] 1.4 Add/update integration tests for `GET /api/tasks` covering: a pending `reconcile_playlist` task returns the playlist name; a pending `download_video` task returns playlist name + video id/title; a task referencing a deleted playlist still returns HTTP 200 with the name omitted.

## 2. Frontend: dependencies and app shell

- [x] 2.1 Add `@radix-ui/react-dialog` and `@radix-ui/react-dropdown-menu` to `web/package.json` and verify `npm install` / build succeeds.
- [x] 2.2 Rework `App.jsx`'s header into: title, top tab bar (Playlists/Tasks), and a right-aligned action area (Add Playlist button + refresh control placeholder), and verify by loading the app and visually confirming the layout at desktop width.
- [x] 2.3 Widen the app's layout (remove/raise the 720px cap in `App.css`) and verify the app renders full-width in a browser at a typical desktop viewport.
- [x] 2.4 Convert `CreatePlaylistForm` into the content of a Radix `Dialog` triggered by the header's "Add Playlist" button, removing it from the default Playlists tab view, and verify: dialog opens on click, submitting creates a playlist (existing `createPlaylist` call unchanged), and the dialog closes on success.

## 3. Frontend: shared refresh control

- [x] 3.1 Replace `usePolling`'s internal `setInterval` with a shared clock (interval id, seconds-remaining state, manual "tick now" function) created once near the app root and exposed via context, and verify existing polling behavior (data still refreshes every ~3s) is unchanged via manual testing.
- [x] 3.2 Update `usePolling` to re-fetch when the shared clock ticks (or is manually bumped) instead of running its own timer, and verify all three views (`PlaylistList`, `TasksView`, `PlaylistDetail`) still refresh correctly.
- [x] 3.3 Add a header refresh indicator (spinner while fetching, seconds-remaining countdown otherwise, manual "refresh now" button that calls the clock's tick function) and verify: countdown decrements visibly, manual click triggers an immediate re-fetch and resets the countdown, and the countdown does not reset when switching tabs.

## 4. Frontend: playlist list — chips and delete

- [x] 4.1 Replace the plain-text `kind · quality` meta in `PlaylistList` with chip elements (reusing/extending the existing `.status-badge` chip styling) and verify visually for both `youtube_linked` and `custom` kinds and each quality value.
- [x] 4.2 Add a Radix `DropdownMenu` ("…") to each playlist list item with a single "Delete" action, and verify it opens without triggering the list item's existing select-playlist click handler.
- [x] 4.3 Add a shared delete-confirmation `AlertDialog` component (used here and in playlist detail, task 5.x) that calls `DELETE /api/playlists/{id}` on confirm, and verify: confirming removes the playlist from the list on next refresh, cancelling leaves it untouched, and a failed delete surfaces an error message.

## 5. Frontend: playlist detail — three-pane redesign

- [x] 5.1 Restructure `PlaylistDetail` into a three-column CSS grid (video list sidebar / player / detail pane) at desktop widths, and verify visually that the player is the largest element and the video list no longer sits above it.
- [x] 5.2 Add a responsive breakpoint below which the sidebar collapses into a toggleable drawer and the detail pane renders below the player instead of beside it, and verify by resizing the browser (or using device emulation) down to phone width.
- [x] 5.3 Add a delete entry point for the current playlist in the detail view (reusing the dropdown/confirmation from 4.2/4.3), and verify deleting from here returns the user to the playlist list.

## 6. Frontend: video detail

- [x] 6.1 Add a shared `formatDateTime` helper (absolute date/time, seconds dropped) and use it for `created_at`/`updated_at` in `VideoDetail`, and verify displayed timestamps show no seconds component.
- [x] 6.2 Add a "Delete" action to `VideoDetail` behind the shared confirmation dialog, wired to the appropriate existing endpoint for the video's playlist kind (custom playlists: `DELETE /api/custom-playlists/{id}/videos/{video_id}`), and verify: the action is hidden or disabled for YouTube-linked playlists (no such endpoint exists for them per the video-listing/custom-playlist-crud specs), and confirms before deleting for custom playlists.

## 7. Frontend: tasks tab

- [x] 7.1 Update `fetchTasks`/`TasksView` to consume the new `playlist_name`/`video_id`/`video_title` fields from `GET /api/tasks` (task 1.3) and render a human-readable description per task type (e.g. "Reconciling playlist X", "Downloading Y in X"), and verify against a task of each type.
- [x] 7.2 Replace the plain-text `status · retries` meta in the task list with chip elements, and verify visually for `pending`/`running` statuses and non-zero retry counts.
- [x] 7.3 Apply the `formatDateTime` helper (task 6.1) to each task's `run_at`, and verify the displayed time shows no seconds component.

## 8. Verification

- [x] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and verify all pass.
- [x] 8.2 Manually exercise the full flow in a browser per this repo's `run` conventions: add a playlist via the dialog, watch it reconcile and download with enriched task context visible, delete a video (custom playlist) and a playlist, and confirm the responsive layout at both desktop and phone width.
