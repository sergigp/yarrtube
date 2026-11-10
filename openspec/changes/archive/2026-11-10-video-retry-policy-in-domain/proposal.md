## Why

Per-video work is retried by mechanisms that don't know about each other, and each follows its own unwritten policy:
- **Video download:** the task executor makes 5 attempts in about 5 h, then dead-letters the task. 24 h later reconcile recovery resets the video, which starts another 5-attempt burst, forever.
- **Subtitles, chapters, quality:** rechecked every 24 h. They settle after 3 failures, but only once the video is old.
- **Thumbnail:** refetched on every source check, forever.
- **AI steps:** they give up on the executor's 5th attempt. Ollama being down is retried on every sync, forever.

Each step also describes its state in its own words. "Unavailable" means "YouTube doesn't have it" for subtitles but "we gave up" for summaries and verdicts. Only the download shows that it is retrying; every other step hides it. On top of that, the executor's `is_last_attempt` leaks into three domain services. Every video that fails for good leaves a dead-lettered task, and the user can neither see nor retry what gave up.

## What Changes

- **One status vocabulary for every per-video step that has a task.** The steps are download, thumbnail, subtitles, chapters, quality check, summary and categorization. Each step is one of:
  - **Pending**: not done yet.
  - **Retrying**: failed 1–10 times, next attempt scheduled.
  - **Done**: succeeded.
  - **Unavailable**: YouTube doesn't have it; final.
  - **Failed**: gave up after 11 attempts.

  The download keeps `IN_PROGRESS` and calls Done `DOWNLOADED`.
- **One retry policy (new `video-retries` capability).**
  - Backoff: 10 min, 1 h, 6 h, then every 24 h; Failed after 11 consecutive failures (~7.3 days).
  - Counts as a failed attempt: any non-permanent download failure, a thumbnail not obtained, a failed late-data check, an AI rejection, and Ollama being down or its model missing.
  - Doesn't count: data not on YouTube yet, a step turned off, a database error.
- **Failed steps re-queue their own task** for the next attempt (`TaskOutcome::RunAgainAt`). Sync schedules due steps whose task got lost. Only infrastructure errors (database, panic) use the executor's backoff and dead-letter.
- **BREAKING (API and storage), stored values:**
  - `VideoStatus`: `ERRORED_RETRYING`/`ERRORED` → `RETRYING`/`FAILED`; `EXCLUDED` → `UNAVAILABLE`.
  - Subtitles: `MISSING`/`FETCHED`/`UNAVAILABLE` → `PENDING`/`DONE`/`UNAVAILABLE`, plus `RETRYING`/`FAILED`.
  - Summary: `MISSING`/`GENERATED`/`UNAVAILABLE` → `PENDING`/`DONE`/`FAILED`.
  - Chapters and quality: their `settled` flags become a status.
  - Thumbnail and categorization: new statuses.
  - Verdict `unavailable` → `failed`.
- **New `steps` object on listed videos:** status, failed attempts, next attempt and last failure reason for every step.
- **New `POST /api/videos/{id}/retry`** and a "Retry now" button in the video detail pane, shown when any step is Retrying or Failed.
  - Every Retrying or Failed step goes back to Pending and its count is cleared.
  - The video's `failed` verdicts are deleted, and queued retries move to now.
  - Done and Unavailable steps are untouched.
- **Removed:** sync no longer recovers errored videos (`ERRORED_RECOVERY_COOLDOWN` goes), and `is_last_attempt` goes from `TaskHandler::handle` and every domain service.
- **Source moves:** Retrying, Failed and Unavailable videos don't block a move, and neither do tasks waiting for a later retry. They resolve the show folder when they run.
- **Web UI:** the status indicator uses the five words. The detail pane lists each Retrying or Failed step, e.g. "Subtitles: retrying, next attempt in 1 hour" or "Summary: failed, gave up after 11 attempts".
- **Migration `0028`:**
  - Maps every stored value to the new vocabulary.
  - `ERRORED` videos get 4 attempts, so 7 more daily tries.
  - Late data settled after at least 3 failures becomes Failed.
  - Drops the replaced columns.

## Capabilities

### New Capabilities
- `video-retries`: the step status vocabulary, the shared backoff, what counts as a failed attempt, re-queueing, giving up after 11 attempts, and retrying a video.

### Modified Capabilities
- `video-download`: the status vocabulary and failure reason; the bounded executor retry and settled-skip rules are replaced.
- `video-sync`: steps are scheduled only when Pending or due; errored recovery is removed.
- `late-video-data`: statuses for late data; failed checks follow the shared backoff and give up after 11.
- `video-subtitles`, `video-chapters`, `video-thumbnails`: the step statuses and shared policy.
- `video-summaries`: summary status; failures follow the shared policy; "Unavailable Summaries Reset" becomes "Failed Summaries Reset".
- `video-categorization`: categorization status, `failed` verdicts, the shared policy; "Unavailable Verdicts Reset" becomes "Failed Verdicts Reset".
- `category-crud`, `video-category-editing`, `preferences`: `unavailable` verdict / summary wording becomes `failed`.
- `task-scheduling`: a handler can re-queue its task; executor retries cover handler errors only.
- `source-relocation`: Retrying, Failed and Unavailable videos and future retries don't block a move.
- `video-listing`: the `steps` object and the new status values.
- `web-ui`: status texts, the failing-steps list and the "Retry now" button.
- `e2e-tests`: the errored-download coverage follows the new flow.

## Impact

- Migration `0028_video_step_statuses.sql`: new status and attempt columns, value mapping, dropped `subtitle_state`, `summary_state`, `chapters_settled`, `quality_settled`.
- Domain:
  - new `StepStatus`, `RetryAttempts`, `StepState`, `VideoStepOutcome`; `LateDataChecks` reworked.
  - `Video`'s per-step fields: `SubtitleState`/`SummaryState` and the settled flags are replaced.
  - `VideoStatus` and `Verdict` change.
  - Every per-video step service returns `VideoStepOutcome`.
  - New `VideoRetrier` service.
- Task queue: `TaskHandler::handle` signature (every handler) and the executor's success path.
- Repositories:
  - video columns and their step-specific writes;
  - `reset_unavailable_summaries` → `reset_failed_summaries`;
  - verdict `delete_unavailable*` → `delete_failed*`, plus `delete_failed_for_video`.
- HTTP: `POST /api/videos/{id}/retry`; `VideoResponse` and `CategoryVideoResponse` (`status`, `steps`).
- Web: `api/types.ts` (`VideoStatus`, `Verdict`, `steps`), `queries.ts`, `VideoDetail.tsx`, `VideoListPane.tsx`, `lib/videoSteps.ts`, the `aVideo` builder, and any component reading `EXCLUDED` or `unavailable`.
- e2e: `helpers/video.js`, `tests/erroredDownload.spec.js`.
- Rollback: the migration drops columns, so downgrading means restoring the database backup taken before upgrading. This goes in the release notes.
- Builds on `simplify-video-write-concurrency` (idea 7, merged in #177): every step-state write goes through `VideoUpdater`, and migration `0027` is taken. Replacing `SubtitleState`/`SummaryState`/settled flags with `StepState` fields touches part of idea 8's `Video` restructuring; idea 8 should start after this lands.
