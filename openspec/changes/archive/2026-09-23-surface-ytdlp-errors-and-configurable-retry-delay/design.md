## Files

- `src/infrastructure/shared/ytdlp.rs`: `download_video` captures stderr instead of inheriting it and returns it on a clean failure.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`: `VideoDownloaderRepository::download` and its implementations return the new outcome type instead of `Option<DownloadedVideo>`.
- `src/domain/services/video_downloader.rs`: uses the failure reason from the new outcome type instead of a hardcoded generic message.
- `src/domain/task/scheduled_task.rs`: `fail`/`retry_delay_seconds` take the base retry delay as a parameter instead of reading a hardcoded constant.
- `src/infrastructure/repositories/task_executor.rs`: `TaskExecutor` holds the base retry delay and passes it to every `fail` call.
- `src/serve.rs`: reads `YARRTUBE_RETRY_BASE_DELAY_SECONDS`, same pattern as `reconcile_interval_seconds()`, and passes it into `TaskExecutor::new`.
- `scripts/run-smoke-tests.sh`: passes a short `YARRTUBE_RETRY_BASE_DELAY_SECONDS` into the container for local smoke runs.

## Types & Signatures

```rust
// src/infrastructure/shared/ytdlp.rs
pub enum DownloadAttempt {
    Succeeded(DownloadedVideo),
    Failed { stderr: Option<String> },
}

pub fn download_video(
    ytdlp_path: &Path,
    video_url: &str,
    desired_filename: &str,
    video_id: &str,
    quality: Quality,
    output_path: &Path,
    existing_folder: Option<&str>,
) -> Result<DownloadAttempt>;

// src/infrastructure/repositories/youtube_video_downloader_repository.rs
pub use crate::infrastructure::shared::ytdlp::DownloadAttempt;

pub trait VideoDownloaderRepository: Send + Sync {
    fn download(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        quality: Quality,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<DownloadAttempt>;

    // fetch_thumbnail: unchanged
}

// src/domain/task/scheduled_task.rs
pub(crate) fn retry_delay_seconds(retries: i64, base_delay_seconds: i64) -> i64;

impl ScheduledTask {
    pub fn fail(
        self,
        error: impl Into<String>,
        now: DateTime<Utc>,
        base_retry_delay_seconds: i64,
    ) -> TaskFailureOutcome;
}

// src/infrastructure/repositories/task_executor.rs
pub struct TaskExecutor {
    repository: Arc<dyn TaskRepository>,
    handlers: HandlerRegistry,
    clock: Arc<dyn Clock>,
    base_retry_delay_seconds: i64,
}

impl TaskExecutor {
    pub fn new(
        repository: Arc<dyn TaskRepository>,
        handlers: HandlerRegistry,
        clock: Arc<dyn Clock>,
        base_retry_delay_seconds: i64,
    ) -> Self;
}

// src/serve.rs
fn retry_base_delay_seconds() -> i64;
```

## Call Stack

Download failure path:
```
VideoDownloaderRepository::download(url, filename, id, quality, output_dir, existing_folder)
  -> ytdlp::download_video(..) -> DownloadAttempt::Failed { stderr }
VideoDownloader::download(video_id, quality, output_dir, is_last_attempt)
  match DownloadAttempt::Failed { stderr }
    -> mark_errored[_retrying](now)
    -> Err(anyhow!(stderr.unwrap_or("yt-dlp failed to download video {video_id}")))
```

Retry delay path:
```
serve.rs: build_application()
  -> retry_base_delay_seconds() -> i64
  -> TaskExecutor::new(repository, handlers, clock, base_retry_delay_seconds)

TaskExecutor::poll_once()/recover_stuck_tasks()
  -> task.fail(error, now, self.base_retry_delay_seconds)
       -> retry_delay_seconds(retries, base_retry_delay_seconds)
       -> run_at = now + Duration::seconds(delay)
```

## Test Plan

- `ytdlp.rs`:
  - `it_should_return_the_stderr_text_on_a_clean_failed_exit` — a fake `yt-dlp` that exits non-zero and writes to stderr; `download_video` returns `DownloadAttempt::Failed { stderr: Some(text) }` with that exact text.
  - `it_should_return_no_stderr_text_when_yt_dlp_writes_nothing_to_stderr` — a fake `yt-dlp` that exits non-zero and writes nothing to stderr; `DownloadAttempt::Failed { stderr: None }`.
  - Existing success-path tests updated to assert `DownloadAttempt::Succeeded(..)` instead of `Some(..)`.
- `youtube_video_downloader_repository.rs`: `it_should_map_a_failed_yt_dlp_process_to_a_failed_attempt_with_its_stderr` replaces `it_should_map_a_failed_yt_dlp_process_to_none`.
- `video_downloader.rs` (existing `download_video_task.rs` test module): `it_should_record_yt_dlps_actual_error_message_when_a_download_fails` — a fake downloader returning `DownloadAttempt::Failed { stderr: Some("HTTP Error 403: Forbidden") }`; asserts the propagated error's message is exactly that text, not the generic one.
- `scheduled_task.rs`:
  - `it_should_use_the_base_delay_passed_in_when_retrying` — `fail(.., base_retry_delay_seconds: 10)` produces a `run_at` computed from 10, not the old hardcoded 150.
  - Existing retry-delay tests updated to pass an explicit base delay.
- `task_executor.rs`: existing retry tests updated to construct `TaskExecutor` with an explicit `base_retry_delay_seconds`; no new behavioral test needed since `scheduled_task.rs` owns the delay math.
- `serve.rs`: no unit test (matches existing untested env-accessor functions like `reconcile_interval_seconds`); verified manually via `scripts/run-smoke-tests.sh`.
