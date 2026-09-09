## 1. Video domain

- [x] 1.1 Add `InProgress`, `Downloaded`, `ErroredRetrying`, `Errored` to
      `VideoStatus` (string round-trip via `as_str`/`parse`), and verify
      with unit tests for each new variant's string representation.
- [x] 1.2 Add `Video::start_download`, `mark_downloaded`,
      `mark_errored_retrying`, `mark_errored` transition methods (consuming
      `self`, returning the updated `Video` with a bumped `updated_at`), and
      verify with unit tests asserting the resulting status and timestamp
      for each transition.

## 2. VideoRepository as CRUD

- [x] 2.1 Add `find(playlist_id, video_id) -> Option<Video>` and
      `update(&Video) -> Result<()>` to `VideoRepository` (writing every
      mutable column, including `status`, unlike `upsert`), implement both
      on `SqliteVideoRepository` and the fake, and verify with tests: `find`
      returns `None` for a missing video and the stored row after `update`
      changes its status.

## 3. Single-video yt-dlp invocation

- [x] 3.1 Extract the `Command::new("yt-dlp")` single-video invocation (and
      output-directory creation) out of
      `infrastructure/client/youtube_downloader_client.rs` into a plain
      function in `infrastructure/shared/` with no trait around it, and
      verify `youtube_downloader_client`'s existing tests still pass calling
      through it (`cargo test youtube_downloader_client`).
- [x] 3.2 Add a `VideoDownloaderRepository` port in
      `infrastructure/repositories/` (`download(video_url, output_dir) ->
      Result<bool>`) implemented by calling the shared function from 3.1,
      plus a fake for tests, and verify with a test against a real `yt-dlp`-shaped
      command double (or the same approach the existing CLI client test uses)
      that a successful/failed process status maps to `Ok(true)`/`Ok(false)`.

## 4. Task attempt visibility

- [x] 4.1 Widen `TaskHandler::handle` to also receive attempt information
      (e.g. `is_last_attempt: bool`, computed by `TaskExecutor` from the
      `ScheduledTask`'s `retries`), update `SyncPlaylistTask` to accept and
      ignore it, and verify `cargo test task_handler task_executor
      sync_playlist_task` all still pass.

## 5. Download task and subscriber

- [x] 5.1 Add `Task::DownloadVideo{playlist_id, video_id}` (type string,
      payload encode/decode) to the `Task` domain enum, and verify with unit
      tests for its type string and payload round-trip, matching
      `Task::SyncPlaylist`'s existing test coverage.
- [x] 5.2 Add `VideoService.download_video(playlist_id, video_id,
      is_last_attempt)`: no-op if the playlist or video no longer exists;
      otherwise `start_download` + `update`, build the video URL and
      `<videos_path>/<playlist.name>/` output dir, call
      `VideoDownloaderRepository`, then `mark_downloaded` on success or
      `mark_errored_retrying`/`mark_errored` (per `is_last_attempt`) on
      failure — returning `Err` on failure so the task queue retries/dead-letters
      it. Verify with unit tests covering: success, failure-with-retries-left,
      failure-on-last-attempt, missing playlist, missing video.
- [x] 5.3 Add `DownloadVideoTask` handler (`tasks/download_video_task.rs`)
      calling `VideoService.download_video`, register it in `tasks::registry`
      for `"download_video"`, and verify with a handler-level test mirroring
      `sync_playlist_task`'s test style.
- [x] 5.4 Add `DownloadVideoOnVideoAdded` subscriber
      (`subscribers/download_video_on_video_added.rs`) that schedules
      `Task::DownloadVideo` for `run_at = now` on `VideoAdded`, register it
      in `subscribers::registry` for `"video_added"`, and verify with a test
      mirroring `sync_playlist_on_playlist_created`'s style (valid payload
      schedules the task; invalid payload no-ops).

## 6. Composition root and configuration

- [x] 6.1 Add `YARRTUBE_VIDEOS_PATH` (default `/videos`) read in `serve.rs`,
      thread it into `VideoService`'s constructor alongside
      `sync_interval_seconds`, wire the new `VideoDownloaderRepository`
      implementation, and verify `cargo build --release` succeeds and
      `serve` starts against a fresh SQLite file with the default and with
      the env var overridden.

## 7. Documentation

- [x] 7.1 Add `YARRTUBE_VIDEOS_PATH` to the configuration table in
      `README.md`, and verify by reading the rendered table for consistency
      with the other rows' format.

## 8. Full verification

- [x] 8.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets
      --all-features --locked -- -D warnings`, and `cargo test --locked`,
      and confirm everything passes, including an end-to-end check (e.g. via
      `serve` + a real or stubbed `yt-dlp` on `PATH`) that adding a video to
      a tracked playlist results in a downloaded file under
      `<videos_path>/<playlist name>/`.
