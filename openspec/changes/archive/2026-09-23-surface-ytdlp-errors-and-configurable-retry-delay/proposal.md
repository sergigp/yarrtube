## Why

Local smoke-test runs (and real background downloads) fail intermittently
when `yt-dlp` hits a transient block from YouTube (e.g. `HTTP Error 403:
Forbidden`). Today that failure is indistinguishable from any other:
`yt-dlp`'s stderr is discarded and the task's recorded error is a generic
"yt-dlp failed to download video X", so diagnosing a flaky run means
digging through raw container logs by hand. Separately, the fixed ~7.5
minute first-retry delay (shared uniformly by every task type) is tuned for
an unattended daemon recovering from hours-long outages, not for a local
smoke-test run that only waits 120 seconds — so a transient block that
would likely clear on retry #2 instead fails the test outright.

## What Changes

- `yt-dlp`'s stderr is captured (not just inherited to the process's own
  stderr) for `download_video` specifically, and its actual reported error
  text is recorded as the task's failure reason instead of a generic
  message. `fetch_thumbnail` and `list_channel_videos` are unchanged — their
  clean-failure cases are already treated as benign, non-retried outcomes,
  not diagnosable task failures.
- The task-retry system's base retry delay becomes configurable via an
  environment variable, defaulting to today's value (production/NAS
  behavior is unchanged unless the variable is set). It still applies
  uniformly to every task type, and the exponential multiplier stays a
  fixed constant (not exposed).
- `scripts/run-smoke-tests.sh` sets that environment variable to a short
  value for local smoke runs, so a transient download failure's retry has
  a realistic chance of completing within the test's existing wait window.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `video-download`: a failed download attempt now records `yt-dlp`'s actual
  reported error text as the failure reason, instead of a generic message.
- `task-scheduling`: the base retry delay used to compute each task's
  next-attempt time is now configurable via an environment variable
  (defaulting to the current value), rather than a fixed constant.

## Impact

- `src/infrastructure/shared/ytdlp.rs`: `download_video` captures stderr
  instead of inheriting it, and returns the captured text on a clean
  failure.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`:
  `VideoDownloaderRepository::download`'s clean-failure case carries the
  captured error text (its return type changes from `Option<DownloadedVideo>`
  to a small result type distinguishing success from a clean failure with
  a reason).
- `src/domain/services/video_downloader.rs`: use the captured `yt-dlp`
  error text (when present) as the failure passed to the task handler,
  instead of the current generic message.
- `src/domain/task/scheduled_task.rs`: read the base retry delay from an
  injected value instead of a hardcoded constant; existing callers
  (`task_executor.rs`, `sqlite_task_repository.rs`, and this module's own
  tests) need to supply it.
- `src/serve.rs`: read the new environment variable the same way
  `YARRTUBE_RECONCILE_INTERVAL_SECONDS` is read today, with a default equal
  to the current hardcoded value.
- `scripts/run-smoke-tests.sh`: set the new environment variable to a short
  value for local runs.
