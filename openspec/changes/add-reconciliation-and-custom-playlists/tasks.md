## 1. Schema & Config Foundations

- [x] 1.1 Add an idempotent `ALTER TABLE playlists ADD COLUMN kind TEXT NOT NULL DEFAULT 'youtube_linked'` and `ALTER TABLE videos ADD COLUMN filename TEXT` to `SqlitePlaylistRepository::new`/`SqliteVideoRepository::new`, swallowing the "duplicate column" error on a re-run; verify by running `cargo test sqlite_playlist_repository sqlite_video_repository` and by constructing each repository twice against the same connection in a test to confirm the second call doesn't error.
- [x] 1.2 Rename the `sync_interval_seconds` config/env var to `reconcile_interval_seconds` (same default of 3600) wherever it's read in `serve.rs`, and update `.env.example`; verify with `cargo build --release` and a manual check that `.env.example` and `serve.rs` agree on the new name.

## 2. Domain Model

- [x] 2.1 Add `domain/playlist/playlist_kind.rs`: a `PlaylistKind` value object shaped like `Quality` (`new`/`as_str`/`Display`, values `youtube_linked`/`custom`); verify with unit tests round-tripping both values and rejecting an invalid one, per this repo's testing conventions (see the `rust-architect` skill).
- [x] 2.2 Add `kind: PlaylistKind` to the `Playlist` entity and thread it through `Playlist::create`; verify existing `domain/playlist/playlist.rs` tests still pass with `kind` added to their fixtures.
- [x] 2.3 Add `VideoId::from_url_or_id`, accepting a bare video ID or a full/short YouTube watch URL; verify with unit tests covering `youtube.com/watch?v=...`, `youtu.be/...`, a bare ID, and a malformed-input rejection.
- [x] 2.4 Add `filename: Option<String>` to the `Video` entity, set by `mark_downloaded` and cleared by a new reset transition (e.g. `reset_for_redownload`) used by filesystem healing; verify with unit tests mirroring the existing status-transition tests in `domain/video/video.rs`.

## 3. Infrastructure: YouTube Single-Video Lookup

- [x] 3.1 Add a `YoutubeVideoRepository` port and `YoutubeApiVideoRepository` implementation hitting the YouTube Data API's `videos` endpoint (`part=snippet&id=<video_id>`) to confirm existence and fetch the title in one call, mirroring `youtube_playlist_repository.rs`'s shape (blocking `reqwest`, API-key query param, `with_base_url` test hook); verify with unit tests against a fake base URL covering found/not-found/API-error cases.
- [x] 3.2 Add a `FakeYoutubeVideoRepository` test double; verify it compiles and is exercised by the `VideoService` tests added in task 5.4.

## 4. Infrastructure: Downloader Filename Capture & Filesystem Matching

- [x] 4.1 Change `ytdlp::download_video` to request yt-dlp's actual output filename via a single explicit `--print` field (e.g. `after_move:filename`) and return it; verify with the existing fake-yt-dlp test support in `ytdlp.rs`, adding a case that asserts the captured filename is returned.
- [x] 4.2 Change `VideoDownloaderRepository::download`'s return type from `anyhow::Result<bool>` to `anyhow::Result<Option<String>>` (`Some(filename)` on success, `None` on a clean yt-dlp failure, `Err` reserved for systemic failures) across the trait, `YtDlpVideoDownloaderRepository`, and `FakeVideoDownloaderRepository`; verify with `cargo build` and updated downloader tests.
- [x] 4.3 Simplify `FilesystemVideoFileRepository::delete` to an exact-filename match (drop the sanitized-title/collision-suffix search) and add a method that lists a playlist output directory's current entries, excluding yt-dlp's in-progress temporary files (`.part`, `.ytdl`, and similar); verify with unit tests covering exact-match deletion, a missing-file no-op, and a listing that excludes temp files.

## 5. Domain Services: Video

- [x] 5.1 Update `VideoService::download_video` to persist the filename returned by the downloader on success (leaving it unset on failure); verify with updated tests in `domain/video/service.rs` and `tasks/download_video_task.rs` asserting `video.filename`.
- [x] 5.2 Add `filename: Option<String>` to `DomainEvent::VideoDeleted` and to `Task::DeleteVideoFile`'s payload (alongside the existing `title`), and update `VideoService::delete_video_file` to delete by the passed-through recorded filename instead of a title-derived guess; verify with updated tests in `domain/event/domain_event.rs`, `domain/task/task.rs`, and `tasks/delete_video_file_task.rs`.
- [x] 5.3 Add `VideoService::reconcile_filesystem(playlist)`: list the playlist's output directory, reset any `Downloaded` video whose recorded filename is missing (clearing `filename`/`quality`, status back to `Pending`) and schedule a fresh `DownloadVideoTask` for it directly, then delete any listed file that isn't the recorded filename of a currently-`Downloaded` video; verify with new unit tests covering: missing file heals and reschedules, orphan file deletes, temp file left alone, matching file causes no action.
- [x] 5.4 Add `VideoService::add_video_to_custom_playlist` and `VideoService::remove_video_from_playlist`, each loading the target playlist first and returning a clear domain error if its `kind` isn't `Custom`; the add path calls `YoutubeVideoRepository` for existence/title before persisting a `Pending` video and publishing `VideoAdded`, and is a no-op if the video is already stored; the remove path deletes the row and publishes `VideoDeleted` (with recorded filename, per 5.2); verify with unit tests covering success, already-a-member no-op, nonexistent playlist, nonexistent/inaccessible YouTube video, and YouTube-linked-playlist rejection for both methods.

## 6. Domain Services: Playlist

- [x] 6.1 Update `PlaylistService::create_playlist` to persist `kind: PlaylistKind::YoutubeLinked` (behavior otherwise unchanged); verify existing `domain/playlist/service.rs` tests pass with the new field asserted.
- [x] 6.2 Add `PlaylistService::create_custom_playlist`, validating the supplied ID as a well-formed UUID, checking it doesn't already identify a playlist of either kind, and never calling the YouTube lookup; verify with new unit tests covering success, malformed UUID, and duplicate ID rejection.

## 7. Reconcile Task Rename & Wiring

- [x] 7.1 Rename `Task::SyncPlaylist` to `Task::ReconcilePlaylist` (`task_type` `sync_playlist` → `reconcile_playlist`) and its payload decode helper; verify updated tests in `domain/task/task.rs`.
- [x] 7.2 Rename `tasks/sync_playlist_task.rs` to `tasks/reconcile_playlist_task.rs` (`SyncPlaylistTask` → `ReconcilePlaylistTask`); update it to run the existing YouTube-membership diff only when `playlist.kind == PlaylistKind::YoutubeLinked`, always call `VideoService::reconcile_filesystem` (task 5.3), and always reschedule itself using the renamed interval config; verify with updated/new tests covering a YouTube-linked playlist (membership diff runs) and a custom playlist (membership diff skipped, filesystem diff still runs).
- [x] 7.3 Update `tasks/mod.rs`'s handler registry and `subscribers/sync_playlist_on_playlist_created.rs` (rename if it reads more naturally, e.g. `reconcile_on_playlist_created.rs`) to reference the renamed task, so playlist creation still schedules one immediate reconcile pass for either kind; verify with updated subscriber tests, including one asserting a newly created custom playlist gets an immediate no-op reconcile pass scheduled/run rather than being skipped.

## 8. HTTP: Custom Playlists

- [x] 8.1 Add an `http/custom_playlists` module with request/response DTOs (create-playlist, add-video) and handlers (`create_custom_playlist`, `add_video`, `remove_video`) calling the services from 5.4/6.2 and mapping each domain error to the HTTP status implied by its spec scenario; verify with new `axum` router tests mirroring `http/playlists/mod.rs`'s style, covering every scenario in `custom-playlist-crud`'s spec.
- [x] 8.2 Register the new routes (`POST /custom-playlists`, `POST /custom-playlists/{id}/videos`, `DELETE /custom-playlists/{id}/videos/{video_id}`) in the app's router; verify with a router-level test hitting the mounted routes end to end.
- [x] 8.3 Add `kind` to `http/playlists/dto.rs`'s `PlaylistResponse` and update `http/playlists/mod.rs`'s existing tests to assert it on both create and list responses; verify with `cargo test http::playlists`.

## 9. Composition Root & Docs

- [x] 9.1 Wire `YoutubeApiVideoRepository`, the renamed task/subscriber registries, the renamed reconcile-interval config, and the new custom-playlists router into `serve.rs::build_application()`; verify with `cargo build --release` and the daemon's existing startup checks.
- [x] 9.2 Update `README.md` to document the new custom-playlist endpoints, the renamed reconcile-interval env var, and the fact that a playlist's output directory is treated as fully system-owned by reconciliation (per design.md's risk note); verify by reading the rendered section back.

## 10. Full Verification

- [x] 10.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; verify all three succeed with zero warnings and zero failures.
