## Context

See proposal.md (Why). Today:
- `SubtitleFetcher` uses a 7-day window counted from `subtitles_first_missing_at`.
- `ChapterFetcher` settles a video on its first check.
- `VideoDownloader` records only the tier, not the height.
- `VideoSyncPlan::for_video` decides from the stored state alone, so every reconcile pass (~3h) re-queues every unsettled check.

## Goals / Non-Goals

**Goals:**
- One domain rule (`LateDataChecks`) used by subtitles, chapters and quality.
- Throttled scheduling.
- Bounded failures.
- In-place quality upgrade for young videos.

**Non-Goals:**
- Merging the three probes into one yt-dlp call.
- Re-probing or re-downloading the existing library: the migration settles chapters and quality.
- Thumbnails, which aren't late data.

## Decisions

- **Check bookkeeping on `Video`, not a separate table.** Each data type gets its own columns. `update_subtitles`, `update_chapters` and `update_quality` keep writing only their own fields, so concurrent tasks can't overwrite each other's.
- **Publish time comes from the check itself.** The probe's `timestamp`/`upload_date` is used, falling back to `video.created_at` when the check failed before reporting one. `Video` doesn't gain a `published_at`.
- **Quality target = the tier's own selector run without downloading** (`-f <tier selector> --print %(height)s --skip-download`). This asks YouTube "what would this tier get now", so there's no per-tier height table for `high`. Upgrade only when the probed height is above the recorded one.
- **The upgrade replaces the file in place** at the recorded filename, then publishes `VideoDownloaded` so the existing subscribers rescan Plex and re-sync. The video stays Downloaded throughout, so the player never sees a missing file.
- **`upgrade_quality` runs in the Download lane** (configurable concurrency) and is keyed `video:<id>`, like a download.
- **Fetch guards stay as they are** (`is_subtitle_fetchable`, `is_chapters_fetchable`); only `VideoSyncPlan` checks `is_due(now)`. A task that's already queued always runs.

## Risks / Trade-offs

- [An old video added recently whose probe fails before reporting a publish time counts as young by `created_at`] → It retries daily for up to 7 days, then follows the 3-attempt limit. This is bounded.
- [A quality upgrade re-downloads the whole file] → Only for young videos below their tier, at most once a day each.
- [Renaming `subtitles_first_missing_at` to `subtitles_last_checked_at` changes its meaning] → It's only used for throttling now, and an approximate value is harmless.

## Migration Plan

`migrations/0021_late_video_data.sql`:
- Renames the subtitles column.
- Adds the chapters/quality check columns and `downloaded_height`.
- Settles chapters where `chapters IS NOT NULL`.
- Settles quality where `status = 'DOWNLOADED'`.

There's no rollback beyond restoring the DB backup. After deploy, the NAS backlog drains in one pass: the queued subtitle tasks run once each and their old videos become Unavailable.

## Files

- `migrations/0021_late_video_data.sql` — new columns, rename, backfill.
- `src/infrastructure/shared/sqlite_migrations.rs` — registers 0021.
- `src/domain/video/late_data_checks.rs` — `LateDataChecks`: the shared young/old, 24h, 3-attempt rule.
- `src/domain/video/late_data_outcome.rs` — `LateDataOutcome`: what one check found.
- `src/domain/video/mod.rs` — exports both.
- `src/domain/shared/quality.rs` — `Quality::target_height`.
- `src/domain/video/video.rs`:
  - New check fields and transitions.
  - `subtitles_first_missing_at` removed.
  - `reset_for_redownload` clears every check.
- `src/domain/video/video_sync_plan.rs` — `for_video(video, now)`: due checks and the `UpgradeQuality` step.
- `src/domain/video/video_sync_step.rs` — `UpgradeQuality` step and task.
- `src/domain/task/task.rs` — `Task::UpgradeQuality`, `upgrade_quality` lane (Download) and key.
- `src/domain/services/subtitle_fetcher.rs` — records every check through `Video::record_subtitles_check`.
- `src/domain/services/chapter_fetcher.rs` — `record_chapters_check` from a `ChaptersProbe` or a failure. Private `record_check` (falls back to `created_at`), mirroring `SubtitleFetcher`.
- `src/domain/services/video_downloader.rs` — records the height and quality settlement, and chapters through `record_chapters_check`.
- `src/domain/services/quality_upgrader.rs` — new service: probe, re-download, replace, record.
- `src/domain/services/mod.rs` — exports `QualityUpgrader`.
- `src/domain/services/video_reconciler.rs` — passes `clock.now()` to `VideoSyncPlan`.
- `src/domain/services/task_view_searcher.rs` — resolves `upgrade_quality` context.
- `src/application/tasks/upgrade_quality_task.rs` — task adapter.
- `src/application/tasks/mod.rs` — exports it.
- `src/serve.rs` — builds `QualityUpgrader` and registers the handler.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs` — `probe_quality`, `fetch_chapters` returning `ChaptersProbe`, and their fakes.
- `src/infrastructure/shared/ytdlp.rs`:
  - Download prints the height.
  - `fetch_chapters` prints the publish time.
  - New `probe_quality`.
- `src/infrastructure/repositories/sqlite_video_repository.rs` — new columns, `update_quality`, and `update_subtitles`/`update_chapters` writing the checks.
- `web/src/lib/tasks.ts`, `web/src/lib/tasks.test.ts` — describe `upgrade_quality` under downloads.
- Existing tests that use `subtitles_first_missing_at` / `mark_subtitles_first_missing` are updated: `fetch_subtitles_task.rs`, `sync_video_task.rs`, `reconcile_*_task.rs`.

## Types & Signatures

```rust
// domain/video/late_data_outcome.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LateDataOutcome {
    Complete,   // found / reached target
    Incomplete, // conclusive: missing or below target
    Failed,     // downloader error or inconclusive warning
}

// domain/video/late_data_checks.rs
pub const YOUNG_FOR: Duration = Duration::days(7);
pub const RECHECK_AFTER: Duration = Duration::hours(24);
pub const MAX_FAILED_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LateDataChecks {
    pub last_checked_at: Option<DateTime<Utc>>,
    pub failed_attempts: u32,
}
impl LateDataChecks {
    pub fn is_due(&self, now: DateTime<Utc>) -> bool;
    /// Records the check (time, +1 failed attempt on Failed).
    pub fn record(self, outcome: LateDataOutcome, now: DateTime<Utc>) -> Self;
    /// Whether, after `record`, the data is final.
    pub fn settles(&self, outcome: LateDataOutcome, published_at: DateTime<Utc>, now: DateTime<Utc>) -> bool;
}

// domain/shared/quality.rs
/// The height that settles a tier's quality; downloads stay uncapped for High.
impl Quality { pub fn target_height(&self) -> u32; } // High 1080, Mid 720, Low 480

// domain/video/video.rs (new/changed fields)
pub subtitle_checks: LateDataChecks,     // replaces subtitles_first_missing_at
pub chapters_checks: LateDataChecks,
pub chapters_settled: bool,
pub downloaded_height: Option<u32>,
pub quality_checks: LateDataChecks,
pub quality_settled: bool,

impl Video {
    /// Incomplete/Failed: records the check, Unavailable once settled.
    pub fn record_subtitles_check(self, outcome: LateDataOutcome, published_at: DateTime<Utc>, now: DateTime<Utc>) -> Self;
    /// `Some(non-empty)` Complete, `Some([])` Incomplete, `None` Failed (keeps
    /// chapters; `Some([])` if settled while unchecked).
    pub fn record_chapters_check(self, chapters: Option<Vec<VideoChapter>>, published_at: DateTime<Utc>, now: DateTime<Utc>) -> Self;
    /// Fetched subtitles: state Fetched, check recorded Complete (was `with_subtitles`).
    pub fn mark_subtitles_fetched(self, subtitle_filename: impl Into<String>, now: DateTime<Utc>) -> Self;
    /// Download or upgrade landed: tier, height, settled if old or target height reached.
    /// `degraded` (SABR reported): a Failed check instead of Complete/Incomplete.
    pub fn record_downloaded_quality(self, quality: Quality, height: Option<u32>, degraded: bool, published_at: DateTime<Utc>, now: DateTime<Utc>) -> Self;
    /// Quality probe found nothing better, or failed (`probed_height` None).
    pub fn record_quality_check(self, probed_height: Option<u32>, published_at: DateTime<Utc>, now: DateTime<Utc>) -> Self;
    pub fn is_quality_checkable(&self) -> bool; // Downloaded && !quality_settled
    // removed: mark_subtitles_first_missing
    // now #[cfg(test)] fixtures only: with_chapters, mark_subtitles_unavailable
}

// domain/video/video_sync_plan.rs
impl VideoSyncPlan { pub fn for_video(video: &Video, now: DateTime<Utc>) -> Self; }

// domain/video/video_sync_step.rs
pub enum VideoSyncStep { Download, FetchThumbnail, FetchSubtitles, FetchChapters, UpgradeQuality }

// domain/task/task.rs
Task::UpgradeQuality { video_id: String, quality: String, output_dir: String } // "upgrade_quality", TaskLane::Download, key video:<id>
impl Task { pub fn decode_upgrade_quality_payload(payload: &str) -> Result<(String, String, String), TaskError>; }
// private: decode_video_quality_payload(payload, task_type), shared with decode_download_video_payload

// infrastructure/shared/ytdlp.rs
pub struct DownloadedVideo { /* ... */ pub height: Option<u32> }
pub struct ChaptersProbe { pub chapters: Vec<VideoChapter>, pub published_at: Option<DateTime<Utc>> }
pub struct QualityProbe { pub height: Option<u32>, pub published_at: Option<DateTime<Utc>>, pub degraded: bool } // degraded: SABR signal on stderr (probe runs without --no-warnings)
pub fn fetch_chapters(ytdlp_path: &Path, video_url: &str) -> Result<ChaptersProbe>;
pub fn probe_quality(ytdlp_path: &Path, video_url: &str, quality: Quality) -> Result<QualityProbe>;

// infrastructure/repositories/youtube_video_downloader_repository.rs
trait VideoDownloaderRepository {
    fn fetch_chapters(&self, video_url: &str) -> anyhow::Result<ChaptersProbe>;
    fn probe_quality(&self, video_url: &str, quality: Quality) -> anyhow::Result<QualityProbe>;
}

// infrastructure/repositories/sqlite_video_repository.rs
trait VideoRepository {
    /// Writes only quality, downloaded_height, quality checks/settled (and updated_at).
    fn update_quality(&self, video: &Video) -> anyhow::Result<()>;
}

// domain/services/quality_upgrader.rs
pub struct QualityUpgrader {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}
pub trait QualityUpgraderApi: Send + Sync {
    /// No-op unless `is_quality_checkable`. Probes the tier's height; when
    /// above the recorded one, re-downloads into staging and replaces the
    /// episode file in place, then records it and publishes
    /// `VideoDownloaded`. Best effort: failures are recorded as a failed
    /// check, never returned.
    fn upgrade(&self, video_id: &VideoRecordId, quality: Quality, output_dir: &Path) -> anyhow::Result<()>;
}

// application/tasks/upgrade_quality_task.rs
pub struct UpgradeQualityTask { quality_upgrader: Arc<QualityUpgrader> }
impl TaskHandler for UpgradeQualityTask { fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()>; }
```

## Call Stack

**Subtitle fetch**
- `FetchSubtitlesTask::handle(payload)` → `SubtitleFetcher::fetch(&video, output_dir)`
  - `video_downloader_repository.probe_captions(url, staging)`
  - Track found → `place_subtitles` → `video.mark_subtitles_fetched(..)` (records Complete) → `update_subtitles`
  - No track, no warning → `video.record_subtitles_check(Incomplete, probe.published_at ∨ created_at, now)` → `update_subtitles`
  - Warning or `Err` → `video.record_subtitles_check(Failed, probe.published_at ∨ created_at, now)` → `update_subtitles`

**Chapters fetch**
- `FetchChaptersTask::handle` → `ChapterFetcher::fetch(&video)`
  - `fetch_chapters(url)`
    - `Ok(probe)` → `video.record_chapters_check(Some(probe.chapters), probe.published_at ∨ created_at, now)`
    - `Err` → `video.record_chapters_check(None, created_at, now)`
  - → `update_chapters`

**Download**
- `VideoDownloader::record_downloaded(video, &downloaded, files, quality, number)`
  - `mark_downloaded(..)`
  - `.record_downloaded_quality(quality, downloaded.height, downloaded.sabr_notice.is_some(), published_at, now)`
  - `.record_chapters_check(downloaded.chapters.clone(), published_at, now)` when the chapters were read
  - → `save`

**Quality upgrade**
- `UpgradeQualityTask::handle(payload)` → `QualityUpgrader::upgrade(&id, quality, output_dir)`
  - `video_repository.find(id)` → guard `is_quality_checkable`
  - `check_and_upgrade`: `video_downloader_repository.probe_quality(url, quality)`
    - `probe.height <= downloaded_height` → `record_quality_check(probe.height unless probe.degraded (then None), published_at, now)` → `update_quality`
    - Higher → `video_downloader_repository.download(url, quality, staging)`
      - `Succeeded` with `downloaded.height > downloaded_height` → `replace`: no recorded filename → nothing; `is_still_downloaded_as` (re-`find`: still Downloaded under the same filename) else nothing → `video_file_repository.move_file(media, output_dir/filename)` → `video.record_downloaded_quality(quality, downloaded.height, downloaded.sabr_notice.is_some(), published_at, now)` → `update_quality` → `event_publisher.publish(VideoDownloaded)`
      - `Succeeded` no taller, or `Failed` → `Err`
  - Any `Err` of `check_and_upgrade` (probe, download, re-`find`, move, `update_quality`) → `record_failed_check` → `record_quality_check(None, created_at, now)` → `update_quality`, logged on failure.
  - Discard staging.

**Sync**
- `VideoReconciler::reconcile` → `schedule_steps(&video, target)` → `VideoSyncPlan::for_video(&video, clock.now())`. Each step is included only when:
  - `FetchSubtitles`: `is_subtitle_fetchable() && subtitle_checks.is_due(now)`
  - `FetchChapters`: `is_chapters_fetchable() && chapters_checks.is_due(now)`
  - `UpgradeQuality`: `is_quality_checkable() && quality_checks.is_due(now)`
- → `task_repository.schedule(step.task_for(..), now)`

## Test Plan

1. Behaviour (acceptance through task adapters)
   - `fetch_subtitles_task`:
     - `it_should_mark_subtitles_unavailable_on_first_check_of_an_old_video` — old video, no track, never checked → Unavailable, last check = now.
     - `it_should_keep_subtitles_missing_for_a_young_video_without_captions` — published 2 days ago → Missing, last check = now, 0 failed attempts.
     - `it_should_count_a_failed_attempt_if_the_probe_warns` — warning → Missing, 1 failed attempt, last check = now.
     - `it_should_count_a_failed_attempt_if_the_probe_fails` — downloader `Err` → Missing, 1 failed attempt.
     - `it_should_mark_subtitles_unavailable_after_three_failed_checks_of_an_old_video` — 2 prior failures + `Err`, created 30 days ago → Unavailable.
     - `it_should_keep_retrying_failed_checks_of_a_young_video` — 5 prior failures, young → Missing, 6 failed attempts.
     - `it_should_fetch_the_original_language_subtitles` (existing) — now also asserts the Complete check (last check = now).
   - `fetch_chapters_task`:
     - `it_should_settle_chapters_found` — chapters reported → recorded, settled.
     - `it_should_keep_empty_chapters_unsettled_for_a_young_video` — `[]`, published 3 days ago → `Some([])`, unsettled, last check = now.
     - `it_should_settle_empty_chapters_of_an_old_video` — `[]`, published 30 days ago → `Some([])`, settled.
     - `it_should_count_a_failed_chapters_attempt` — `Err` → chapters unchanged, 1 failed attempt.
     - `it_should_settle_chapters_as_none_after_three_failed_checks_of_an_old_video` — `None` + 2 failures, created 30 days ago → `Some([])`, settled.
     - `it_should_skip_settled_chapters` — settled → no downloader call, unchanged.
   - `download_video_task`:
     - `it_should_record_the_downloaded_height` — height 1080 recorded.
     - `it_should_leave_quality_unsettled_for_a_young_video_below_its_cap` — mid, 360, published 1h ago → unsettled, last check = now.
     - `it_should_settle_quality_at_the_tier_cap` — mid, 720 → settled.
     - `it_should_settle_quality_of_an_old_video` — published 30 days ago → settled.
     - `it_should_settle_high_quality_at_1080` — high, 1080, published 1h ago → settled.
     - `it_should_count_a_degraded_download_as_a_failed_quality_check` — old video, SABR notice, 360 → unsettled, 1 failed attempt.
     - `it_should_leave_empty_chapters_unsettled_for_a_young_video` — `[]` on download, young → unsettled.
   - `upgrade_quality_task`:
     - `it_should_replace_the_file_with_a_higher_quality_download` — probe 1080 > 360 → file moved over the same name, height 1080, settled (High target reached), subtitles/chapters/watch unchanged, `VideoDownloaded` event in the outbox.
     - `it_should_record_the_check_if_nothing_better_is_available` — probe 360 = 360 → no download, last check = now, unsettled.
     - `it_should_settle_quality_once_the_video_is_old` — published 8 days ago, probe equal → settled.
     - `it_should_count_a_failed_attempt_if_the_probe_fails` — `Err` → 1 failed attempt, file unchanged.
     - `it_should_keep_the_file_if_the_redownload_fails` — download `Failed` → file and fields unchanged, 1 failed attempt, `Ok`.
     - `it_should_count_a_degraded_probe_as_a_failed_check` — old video, probe 360 degraded → nothing downloaded, 1 failed attempt, unsettled.
     - `it_should_count_a_degraded_upgrade_as_a_failed_check` — 8 days old, probe 1080, re-download 720 with SABR → replaced, height 720, 1 failed attempt, unsettled.
     - `it_should_keep_the_file_if_the_redownload_is_no_better` — probe 1080, download 240 < 360 → no move, 1 failed attempt, `Ok`.
     - `it_should_skip_the_upgrade_if_the_video_was_reset_meanwhile` — video reset during the download → no move, nothing recorded.
     - `it_should_skip_the_upgrade_if_the_video_was_redownloaded_meanwhile` — still Downloaded, filename changed → no move, nothing recorded.
     - `it_should_skip_a_video_with_settled_quality` — no downloader call.
   - `sync_video_task`:
     - `it_should_not_schedule_subtitles_checked_less_than_a_day_ago` — Missing, last check 3h ago → nothing.
     - `it_should_schedule_subtitles_checked_more_than_a_day_ago` — last check 25h ago → `FetchSubtitles`.
     - `it_should_schedule_chapters_of_a_young_video_checked_a_day_ago` — `Some([])`, unsettled, 25h → `FetchChapters`.
     - `it_should_not_schedule_settled_chapters` — `Some([])`, settled → nothing.
     - `it_should_schedule_a_quality_check_when_due` — unsettled, 25h → `UpgradeQuality` with the owner's quality and output dir.
     - `it_should_not_schedule_a_quality_check_right_after_download` — last check 10 min ago → nothing.
     - `it_should_reset_every_check_on_redownload` — broken file → all checks cleared and unsettled, height `None`.
   - `http/tasks`:
     - `it_should_list_an_upgrade_quality_task_with_its_video_context` — title + playlist name resolved.
2. Infrastructure
   - `sqlite_migrations`:
     - `it_should_settle_checked_chapters_and_downloaded_quality` — pre-0021 rows → `chapters_settled` / `quality_settled` backfilled, unchecked chapters unsettled.
     - `it_should_keep_the_first_missing_time_as_the_last_subtitles_check` — renamed column keeps its value.
   - `sqlite_video_repository`:
     - `it_should_round_trip_late_data_checks` — insert/find preserves every new field.
     - `it_should_update_only_quality_fields` — `update_quality` leaves the watch position and subtitles untouched.
   - `ytdlp`:
     - `it_should_report_the_downloaded_height` — fake script prints the height → `DownloadedVideo.height`.
     - `it_should_report_the_chapters_publish_time` — `fetch_chapters` → `ChaptersProbe.published_at`.
     - `it_should_probe_the_height_for_a_quality` — runs the tier's `-f` selector with `--skip-download`, parses height and publish time.
     - `it_should_fail_a_quality_probe_on_a_clean_failure` — non-zero exit → `Err`.
     - `it_should_report_a_degraded_quality_probe` — SABR warning on stderr → `QualityProbe.degraded`.
     - (private `published_at_from_tail` shared by `fetch_chapters` and `probe_quality`.)
   - `task`:
     - `it_should_run_upgrade_quality_in_the_download_lane_exclusive_per_video` — lane Download, key `video:<id>`, dedupes running.
     - `it_should_read_the_output_dir_of_an_upgrade_quality` — `output_dir_of` returns the payload's output dir.
   - `web/src/lib/tasks.test.ts`:
     - describes `upgrade_quality` as a quality check under downloads, with a placeholder when unresolved.
