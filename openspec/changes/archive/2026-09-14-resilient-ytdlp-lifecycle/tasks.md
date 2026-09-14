## 1. Bundle a known-good `yt-dlp` binary into the image

- [x] 1.1 Add a build step to `Dockerfile` that fetches the latest stable `yt-dlp` Linux binary (same GitHub releases API and asset name `yt-dlp_linux` used by `update-ytdlp`) and installs it into the image at the new default install path (see 2.1), executable. Verify with `docker build .` succeeding and `docker run --rm <image> /app/bin/yt-dlp --version` (adjust path once 2.1 lands) printing a version.

## 2. Unify the install path and invocation (fixes the permission bug)

- [x] 2.1 Change `DEFAULT_YTDLP_PATH` in `src/cli/ytdlp_update.rs` to a path under `/app` (e.g. `/app/bin/yt-dlp`), which `docker-entrypoint.sh` already `chown -R $PUID:$PGID`s. Verify existing tests in that module still pass: `cargo test ytdlp_update`.
- [x] 2.2 Change `download_video` in `src/infrastructure/shared/ytdlp.rs` to invoke the binary at the configured path instead of `Command::new("yt-dlp")` (a bare `PATH` lookup). Verify with a test asserting it invokes the configured path, and that a missing binary at that path still surfaces the existing "not found" error: `cargo test ytdlp`.
- [x] 2.3 Thread the configured path from the composition root (`serve.rs::build_application`, currently `Arc::new(YtDlpVideoDownloaderRepository)`) into `YtDlpVideoDownloaderRepository`. Verify `cargo build` compiles with the new wiring and existing `youtube_video_downloader_repository` tests pass: `cargo test youtube_video_downloader_repository`.
- [x] 2.4 Point the Dockerfile's bundled-binary install (1.1) at the same path chosen in 2.1, so the build-time binary and the runtime-installed binary always agree. Verify `docker build .` then `docker run --rm <image> ls -l <path>` shows an executable file at that path.
- [x] 2.5 Update `README.md`'s documented `YTDLP_PATH` default and its "the container's own database and `yt-dlp` binary are recreated on restart" note to describe the bundled-binary floor. Verify by re-reading the relevant section for accuracy.

## 3. Recurring self-update task

- [x] 3.1 Add a new task type (e.g. `Task::UpdateYtdlp`) alongside the existing variants in `src/domain/task/`, following the same payload encode/decode shape as `Task::ReconcilePlaylist`. Verify with a round-trip encode/decode unit test.
- [x] 3.2 Add a handler (e.g. `src/tasks/update_ytdlp_task.rs`) that runs the update and unconditionally reschedules the next occurrence ~1h out, regardless of whether this attempt succeeded — it must not go through `ScheduledTask::fail`/dead-letter, since a failed update is not a permanent failure of a unit of work. Register it in `tasks::registry()`. Verify with unit tests: a failed update still reschedules the next occurrence; a successful update also reschedules the next occurrence (mirror `reconcile_playlist`'s existing self-reschedule test pattern).
- [x] 3.3 Schedule the first occurrence of the recurring task once at daemon startup (`serve.rs`), in addition to (not replacing) the existing immediate `run_startup_ytdlp_update()` call. Verify with a test or log assertion that an `update_ytdlp` task is scheduled after startup.

## 4. Retry-count-based backoff

- [x] 4.1 Replace `RETRY_DELAY_SECONDS` in `src/domain/task/scheduled_task.rs` with a delay computed from the post-increment `retries` count (e.g. `base_seconds * multiplier.pow(retries)`), tuned so 5 attempts span roughly 5 hours end-to-end. `MAX_ATTEMPTS` stays 5. Verify with unit tests asserting `run_at` grows between successive `ScheduledTask::fail` calls, extending the existing tests in that module.
- [x] 4.2 Search for and update any test elsewhere that asserts the old fixed 30-second retry delay (e.g. in `task_executor.rs`'s retry tests) to match the new formula. Verify with `cargo test task_executor`.

## 5. Reconcile recovers permanently-errored videos

- [x] 5.1 Extend `reconcile_filesystem` in `src/domain/video/service.rs` to also find videos with status `Errored` for the playlist, reset them via the existing `reset_for_redownload`, and schedule a `Task::DownloadVideo` — the same recovery shape already used there for a `Downloaded` video whose file went missing. Verify with a unit test asserting an `Errored` video is reset to `Pending` (filename/quality cleared) and a download task is scheduled after a reconcile pass.
- [x] 5.2 Add a unit test asserting a video that errors again after being reconcile-recovered is recovered again on a later reconcile pass, with no cap on recovery count, matching the new `playlist-reconciliation` spec scenario.

## 6. Full verification

- [x] 6.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`, and verify all pass.
- [x] 6.2 Build the image locally (`docker build .`) and manually verify, with `PUID`/`PGID` set the way the NAS deployment does, that startup no longer logs `Failed to create temp file`, and that `download` succeeds using only the bundled binary when network access to GitHub is blocked.
- [x] 6.3 Run `openspec validate resilient-ytdlp-lifecycle --strict` and verify it reports the change as valid before archiving.
