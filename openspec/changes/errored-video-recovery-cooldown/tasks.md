## 1. JavaScript runtime in the image

- [x] 1.1 Copy `/deno` from `denoland/deno:bin` into `/usr/local/bin/deno` in the runtime stage of `Dockerfile`. Verified with a bookworm-slim image containing the same yt-dlp build: `yt-dlp -v` reports `JS runtimes: deno-…` and video `3812m0ubckk` resolves as `public` instead of `This video is not available`
- [x] 1.2 Build the full image (`docker build .`) and run `docker run --rm --entrypoint /app/bin/yt-dlp <image> -v --simulate https://www.youtube.com/watch?v=3812m0ubckk`. Verify the output lists deno under `JS runtimes` and ends without `ERROR`

## 2. Errored video recovery cooldown

- [x] 2.1 Add a migration with a nullable `videos.last_errored_at` column, and add `Video.last_errored_at` (`src/domain/video/video.rs`): set by `mark_errored`, kept by `reset_for_redownload`, and mapped in `SqliteVideoRepository` insert, update and row mapping. Base `is_due_for_recovery` on it, with a named 24h cooldown constant: true only for `Errored` videos whose `last_errored_at` is `None` or at least 24h before `now`. Verify with unit tests covering errored ≥24h (true), exactly 24h (true), <24h (false), errored with no recorded timestamp (true) and non-errored statuses (false), and with a repository test that round-trips `last_errored_at`
- [x] 2.2 Use the predicate in `PlaylistVideoReconciler`'s errored-video recovery loop instead of the plain `status == Errored` filter. Update `it_should_retry_permanently_errored_videos` in `src/application/tasks/reconcile_playlist_task.rs` to an errored-≥24h video, add a test asserting that an errored-<24h video stays `Errored` with no download task scheduled, and verify both pass
- [ ] 2.3 Use the same predicate in `ChannelVideoReconciler`'s errored-video recovery loop. Mirror the two tests in `src/application/tasks/reconcile_channel_task.rs` and verify both pass

## 3. Task dispatch order

- [ ] 3.1 Change `list_eligible` in `src/infrastructure/repositories/sqlite_task_repository.rs` to `ORDER BY run_at ASC, id ASC`. Add repository tests verifying that a later-scheduled task with an earlier `run_at` is listed first, and that tasks with equal `run_at` are listed in id order

## 4. Verification

- [ ] 4.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo test --locked`, and verify all pass
