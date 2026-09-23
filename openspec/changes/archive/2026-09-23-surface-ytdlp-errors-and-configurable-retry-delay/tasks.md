## 1. Surface yt-dlp's real failure reason

- [x] 1.1 In `src/infrastructure/shared/ytdlp.rs`, add `DownloadAttempt { Succeeded(DownloadedVideo), Failed { stderr: Option<String> } }`, switch `download_video`'s `.stderr(Stdio::inherit())` to `.stderr(Stdio::piped())`, and return `Failed { stderr }` (captured stderr, trimmed, `None` if empty) on a clean non-zero exit instead of `Ok(None)`; verify via `it_should_return_the_stderr_text_on_a_clean_failed_exit` and `it_should_return_no_stderr_text_when_yt_dlp_writes_nothing_to_stderr`.
- [x] 1.2 Update `download_video`'s existing tests and callers in the same file to match `DownloadAttempt` instead of `Option<DownloadedVideo>`; verify `cargo test --locked` passes for `ytdlp.rs`.
- [x] 1.3 In `src/infrastructure/repositories/youtube_video_downloader_repository.rs`, change `VideoDownloaderRepository::download`'s return type to `anyhow::Result<DownloadAttempt>` and update `YtDlpVideoDownloaderRepository` and `FakeVideoDownloaderRepository` accordingly; replace `it_should_map_a_failed_yt_dlp_process_to_none` with `it_should_map_a_failed_yt_dlp_process_to_a_failed_attempt_with_its_stderr`.
- [x] 1.4 In `src/domain/services/video_downloader.rs`, match on `DownloadAttempt` in `download`: on `Failed { stderr }`, use `stderr.unwrap_or_else(|| format!("yt-dlp failed to download video {video_id}"))` as the error message instead of the current hardcoded string; verify via a new `download_video_task.rs` test `it_should_record_yt_dlps_actual_error_message_when_a_download_fails`.
- [x] 1.5 Run `cargo clippy --all-targets --all-features --locked -- -D warnings` and `cargo fmt --all -- --check` and fix any fallout from the signature change.

## 2. Configurable retry base delay

- [x] 2.1 In `src/domain/task/scheduled_task.rs`, change `retry_delay_seconds` to take `base_delay_seconds: i64` as a parameter (dropping the `BASE_RETRY_DELAY_SECONDS` const), and change `ScheduledTask::fail` to take `base_retry_delay_seconds: i64` and pass it through; keep `RETRY_DELAY_MULTIPLIER` and `MAX_ATTEMPTS` as hardcoded consts; update this module's existing tests to pass an explicit base delay and add `it_should_use_the_base_delay_passed_in_when_retrying`.
- [x] 2.2 In `src/infrastructure/repositories/task_executor.rs`, add a `base_retry_delay_seconds: i64` field to `TaskExecutor`, accept it in `TaskExecutor::new`, and pass it to every `.fail(..)` call in `poll_once` and `recover_stuck_tasks`; update this module's existing tests' `TaskExecutor::new` calls to supply an explicit value.
- [x] 2.3 In `src/serve.rs`, add `const DEFAULT_RETRY_BASE_DELAY_SECONDS: i64 = 150;` and a `retry_base_delay_seconds()` accessor reading `YARRTUBE_RETRY_BASE_DELAY_SECONDS` the same way `reconcile_interval_seconds()` reads its own variable, and pass its result into the `TaskExecutor::new` call in `build_application`.
- [x] 2.4 Run `cargo test --locked` and confirm the full suite passes with the new parameter threaded through.

## 3. Wire the smoke suite to the short retry delay

- [x] 3.1 In `scripts/run-smoke-tests.sh`, add `-e "YARRTUBE_RETRY_BASE_DELAY_SECONDS=${SMOKE_RETRY_BASE_DELAY_SECONDS:-10}"` to the `docker run` invocation, documented the same way other `SMOKE_*` overrides are in `smoke-tests/README.md`'s optional-overrides table.
- [x] 3.2 Run `./scripts/run-smoke-tests.sh` locally at least once and confirm it still passes end-to-end with the new env var wired through.
