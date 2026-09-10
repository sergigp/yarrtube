## 1. Domain event and task types

- [x] 1.1 Add `DomainEvent::VideoDeleted { playlist_id, video_id, title, was_downloaded }` in `src/domain/event/domain_event.rs` with `event_type()` returning `"video_deleted"` and a matching `payload()`; verify with a unit test asserting the stable type/payload mapping.
- [x] 1.2 Add `Task::DeleteVideoFile { playlist_id, video_id, title }` in `src/domain/task/task.rs` with `task_type()` returning `"delete_video_file"`, a matching `payload()`, and a `decode_delete_video_file_payload` helper; verify with unit tests for the mapping, decode round-trip, and rejection of a malformed payload.

## 2. `VideoFileRepository` port

- [x] 2.1 Add `VideoFileRepository` trait in a new `src/infrastructure/repositories/video_file_repository.rs`, with `delete(&self, output_dir: &Path, filename_stem: &str, video_id: &str) -> anyhow::Result<bool>`, matching a file whose stem is exactly `filename_stem` or `{filename_stem} [{video_id}]` (any extension) and deleting every match; treat a missing `output_dir` as "nothing to delete" (`Ok(false)`), not an error.
- [x] 2.2 Implement it (e.g. `FilesystemVideoFileRepository`) and add a `FakeVideoFileRepository` for tests, alongside the trait in the same file, following the existing `VideoDownloaderRepository` pattern; verify with unit tests covering: exact-stem match deleted, bracketed-video-id match deleted, no match is a no-op, missing directory is a no-op.

## 3. `VideoService`

- [x] 3.1 Add `video_file_repository: Arc<dyn VideoFileRepository>` to `VideoService`'s fields and constructor.
- [x] 3.2 Add `VideoService::delete_video_file(playlist_id, video_id, title)`: no-op if the playlist no longer exists, otherwise resolve the output directory and sanitized filename and call `video_file_repository.delete(...)`, logging the outcome; verify with unit tests for "file deleted when present," "no-op when playlist gone," and "no-op when no matching file."
- [x] 3.3 Update `VideoService::sync_playlist_videos`'s removal loop to publish `DomainEvent::VideoDeleted { playlist_id, video_id, title: stored.title.clone(), was_downloaded: stored.status == VideoStatus::Downloaded }` right after each `video_repository.delete(...)` call; verify with a unit test asserting the event is published with the correct `was_downloaded` value for both a downloaded and a non-downloaded removed video.

## 4. Subscriber and task handler

- [x] 4.1 Add `src/subscribers/delete_video_file_on_video_deleted.rs`: parses the `VideoDeleted` payload, schedules `Task::DeleteVideoFile` (via `task_repository.schedule(..., clock.now())`) only when `was_downloaded` is true, no-ops otherwise or on invalid IDs; verify with unit tests for "schedules when downloaded," "does not schedule when not downloaded," and "no-ops on invalid payload IDs."
- [x] 4.2 Register it in `subscribers::registry` for event type `"video_deleted"`.
- [x] 4.3 Add `src/tasks/delete_video_file_task.rs`: decodes the `DeleteVideoFile` payload, no-ops on invalid IDs, delegates to `VideoService::delete_video_file`; verify with unit tests mirroring `DownloadVideoTask`'s (successful deletion, invalid payload IDs no-op, malformed payload rejected).
- [x] 4.4 Register it in `tasks::registry` for task type `"delete_video_file"`.

## 5. Wiring

- [x] 5.1 Update `serve.rs::build_application()` to construct the concrete `VideoFileRepository` implementation and pass it into `VideoService::new`.
- [x] 5.2 Verify `cargo build --release` succeeds with the updated `VideoService` constructor signature at every call site (including tests).

## 6. Final verification

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; confirm all three are clean.
- [ ] 6.2 Manually verify end-to-end: track a playlist, let a video download, remove it from the playlist on YouTube, trigger a sync, and confirm the downloaded file is deleted from `videos_path/<playlist name>/`.
