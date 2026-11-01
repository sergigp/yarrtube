# Design

## Files

- `migrations/0020_video_chapters.sql` — adds nullable `videos.chapters` (JSON text; NULL = never checked, `[]` = none).
- `src/domain/video/video_chapter.rs` — `VideoChapter` value type (one chapter).
- `src/domain/video/video.rs` — `chapters` field, `with_chapters`, `is_chapters_fetchable`.
- `src/domain/video/video_sync_step.rs` — `FetchChapters` step → `Task::FetchChapters`.
- `src/domain/video/video_sync_plan.rs` — schedules `FetchChapters` when `is_chapters_fetchable`.
- `src/domain/task/task.rs` — `Task::FetchChapters`, payload decode, lane (`Subtitles`) and `video:<id>` exclusivity key.
- `src/domain/services/chapter_fetcher.rs` — `ChapterFetcher`: fetch + record chapters for one video.
- `src/domain/services/video_downloader.rs` — records `DownloadedVideo::chapters` in `record_downloaded`.
- `src/domain/services/mod.rs` — export `ChapterFetcher`.
- `src/infrastructure/shared/ytdlp.rs` — `--print %(chapters)j` + `--embed-chapters` in `download_video`; new `fetch_chapters`; `parse_chapters`.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs` — `fetch_chapters` on the port, real impl, fake.
- `src/infrastructure/repositories/sqlite_video_repository.rs` — read/write `chapters` in `save`/`update`/row mapping; new `update_chapters`.
- `src/application/tasks/fetch_chapters_task.rs` — `FetchChaptersTask` handler.
- `src/application/tasks/mod.rs` — register `fetch_chapters`.
- `src/serve.rs` — build `ChapterFetcher`, pass to the registry.
- `src/application/http/videos/dto.rs` — `ChapterResponse`, `chapters` on `VideoResponse` and `CategoryVideoResponse`.
- `src/domain/services/task_view_searcher.rs` — resolve `fetch_chapters` to the video's title and owner name.
- `src/serve.rs` test — `fetch_chapters` in the registered task types.
- `web/src/api/types.ts` — `Chapter`, `chapters` on `Video` and `CategoryVideo`.
- `web/src/lib/tasks.ts` — describe `fetch_chapters`, family `downloads`, kind `download`.
- `web/src/test/helpers.tsx` — `aVideo` / `aCategoryVideo` default `chapters: []`.

## Types & Signatures

```rust
// domain/video/video_chapter.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoChapter {
    pub start_seconds: i64,
    pub end_seconds: i64,
    pub title: String,
}

// domain/video/video.rs
pub struct Video {
    // ...
    /// `None` = never checked; `Some(vec![])` = checked, no chapters.
    pub chapters: Option<Vec<VideoChapter>>,
}
impl Video {
    /// Sets only `chapters` and `updated_at`.
    pub fn with_chapters(self, chapters: Vec<VideoChapter>, now: DateTime<Utc>) -> Self;
    /// Downloaded and `chapters.is_none()`.
    pub fn is_chapters_fetchable(&self) -> bool;
}

// domain/video/video_sync_step.rs
pub enum VideoSyncStep { Download, FetchThumbnail, FetchSubtitles, FetchChapters }

// domain/task/task.rs
pub enum Task {
    // ...
    FetchChapters { video_id: String },
}
impl Task {
    pub fn decode_fetch_chapters_payload(payload: &str) -> Result<String, TaskError>;
}

// domain/services/chapter_fetcher.rs
pub struct ChapterFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    clock: Arc<dyn Clock>,
}
pub trait ChapterFetcherApi: Send + Sync {
    /// No-op unless `video.is_chapters_fetchable()`. Fetches and records
    /// the chapters via `update_chapters`; any failure is logged and
    /// swallowed, leaving the chapters unchecked.
    fn fetch(&self, video: &Video);
}

// infrastructure/shared/ytdlp.rs
pub struct DownloadedVideo {
    // ...
    /// `None` when the printed chapters line could not be parsed.
    pub chapters: Option<Vec<VideoChapter>>,
}
pub fn fetch_chapters(ytdlp_path: &Path, video_url: &str) -> Result<Vec<VideoChapter>>;
/// `NA` / `null` → `Some(vec![])`; JSON array → chapters (times floored to
/// whole seconds); anything else → `None`.
fn parse_chapters(line: &str) -> Option<Vec<VideoChapter>>;

// youtube_video_downloader_repository.rs
pub trait VideoDownloaderRepository {
    // ...
    /// The video's chapters, downloading nothing. `Err` on any failure.
    fn fetch_chapters(&self, video_url: &str) -> anyhow::Result<Vec<VideoChapter>>;
}

// sqlite_video_repository.rs
pub trait VideoRepository {
    // ...
    /// Writes only `chapters` (and `updated_at`).
    fn update_chapters(&self, video: &Video) -> anyhow::Result<()>;
}

// application/tasks/fetch_chapters_task.rs
pub struct FetchChaptersTask {
    video_repository: Arc<dyn VideoRepository>,
    chapter_fetcher: Arc<ChapterFetcher>,
}

// application/http/videos/dto.rs
#[derive(Serialize)]
pub struct ChapterResponse { pub start_seconds: i64, pub end_seconds: i64, pub title: String }
// VideoResponse / CategoryVideoResponse gain: pub chapters: Vec<ChapterResponse>  (None → [])
```

```ts
// web/src/api/types.ts
export interface Chapter { start_seconds: number; end_seconds: number; title: string }
// Video, CategoryVideo: chapters: Chapter[]
```

## Call Stack

**Download (new videos)**
```
DownloadVideoTask::handle(payload)
  VideoDownloader::download(video_id, quality, output_dir)
    VideoDownloaderRepository::download(url, quality, staging_dir)
      ytdlp::download_video  -- yt-dlp ... --embed-chapters --print %(chapters)j --print %(timestamp)s ... --print after_move:filename
        parse_chapters(line_before_last(&lines, 4))  -> DownloadedVideo { chapters, .. }
    VideoDownloader::record_downloaded(video, &downloaded, ..)
      video.mark_downloaded(..)  then  .with_chapters(c, now) when downloaded.chapters is Some(c)
      VideoRepository::update(&video)
```
`%(chapters)j` is the first `--print`, so the existing from-the-end line positions don't shift.

**Backfill**
```
SyncVideoTask::handle -> VideoReconciler::reconcile(video_id)
  schedule_steps -> VideoSyncPlan::for_video(video)   // + FetchChapters if is_chapters_fetchable
    TaskRepository::schedule(Task::FetchChapters { video_id }, now)   // deduped by video:<id>
FetchChaptersTask::handle(payload)
  VideoRepository::find(video_id)            // gone -> Ok(())
  ChapterFetcher::fetch(&video)
    VideoDownloaderRepository::fetch_chapters(url)
      ytdlp::fetch_chapters -- yt-dlp --skip-download --quiet --no-warnings --print %(chapters)j <url>
    video.with_chapters(chapters, now)
    VideoRepository::update_chapters(&video)
```
Always returns `Ok(())` (best-effort, like thumbnails): a failed fetch is retried by the next reconcile pass, not the queue.
After upgrading, the backfill queues one chapters fetch per downloaded video in the Subtitles lane, so new downloads' subtitles wait behind it once, as with the 0010 subtitle backfill; accepted.

**Listing**
```
list_videos_for_playlist / list_videos_for_channel / list_videos_for_category
  ... VideoView -> VideoResponse::from (chapters.unwrap_or_default() -> Vec<ChapterResponse>)
  CategoryVideoView -> CategoryVideoResponse::from (copies VideoResponse.chapters)
```

## Test Plan

### 1. Behaviour tests

`application/tasks/download_video_task.rs`
1. `it_should_record_the_chapters_of_a_downloaded_video` — fake download returns 3 chapters → stored video has exactly those chapters.
2. `it_should_record_no_chapters_for_a_downloaded_video_without_them` — fake returns `Some(vec![])` → stored `Some(vec![])`.
3. `it_should_leave_chapters_unchecked_if_unreadable_on_download` — fake returns `None` → video Downloaded, chapters `None`.

`application/tasks/sync_video_task.rs`
4. `it_should_schedule_the_chapters_fetch_of_a_downloaded_video` — Downloaded, chapters `None` → `fetch_chapters` task scheduled.
5. `it_should_not_schedule_a_chapters_fetch_if_already_checked` — Downloaded, chapters `Some(vec![])` → none scheduled.
6. `it_should_not_schedule_a_chapters_fetch_for_a_pending_video` — Pending, chapters `None` → none scheduled.

`application/tasks/fetch_chapters_task.rs`
7. `it_should_record_the_fetched_chapters` — fake returns chapters → stored, other fields unchanged.
8. `it_should_record_no_chapters_if_the_video_has_none` — fake returns `[]` → stored `Some(vec![])`.
9. `it_should_leave_chapters_unchecked_if_the_fetch_fails` — fake `Err` → task `Ok`, video unchanged.
10. `it_should_skip_if_chapters_already_checked` — `Some(..)` → fake not called.
11. `it_should_skip_if_the_video_is_gone` — no record → `Ok`, fake not called.
12. `it_should_keep_a_playback_position_recorded_during_the_fetch` — position updated after `find` (fake hook) → position kept, chapters recorded.

`application/http/videos/mod.rs`
13. `it_should_include_the_chapters_when_listing_playlist_videos` — video with 2 chapters → response `chapters` equals them.
14. `it_should_list_unchecked_chapters_as_empty` — chapters `None` → `chapters: []`.
15. `it_should_include_the_chapters_when_listing_category_videos` — category entry carries the chapters.

`domain/task/task.rs`
16. `it_should_run_fetch_chapters_in_the_subtitles_lane_exclusive_per_video` — lane `Subtitles`, key `video:<id>`.

`application/http/tasks/mod.rs`
16a. `it_should_list_a_fetch_chapters_task_with_video_and_playlist_names` — task view payload has `video_title` and `playlist_name`.

`web/src/lib/tasks.test.ts` (Vitest)
16b. `describes chapter fetches` — "Fetching chapters of Intro in Chan", placeholder when unresolved; family `downloads`, kind `download`.

### 2. Infrastructure tests

`infrastructure/shared/ytdlp.rs` (fake `yt-dlp` script)
17. `it_should_parse_the_printed_chapters_of_a_download` — script prints a chapters JSON line before timestamp/date/duration/filename → `DownloadedVideo.chapters`, other fields still parsed.
18. `it_should_report_no_chapters_when_yt_dlp_prints_na` — `NA` → `Some(vec![])`.
19. `it_should_report_unreadable_chapters_as_unchecked` — garbage line → `None`, download still `Succeeded`.
20. `it_should_pass_the_embed_chapters_flag_to_yt_dlp` — args contain `--embed-chapters`.
21. `it_should_fetch_the_chapters_without_downloading` — `fetch_chapters` passes `--skip-download`, parses the printed line, floors fractional seconds.
22. `it_should_fail_the_chapters_fetch_on_a_clean_failed_exit` — non-zero exit → `Err`.

`infrastructure/repositories/sqlite_video_repository.rs`
23. `it_should_round_trip_chapters_none_empty_and_some` — `None`, `Some([])`, `Some([..])` persist distinctly.
24. `it_should_update_only_the_chapters` — `update_chapters` leaves other columns as stored.

`infrastructure/repositories/youtube_video_downloader_repository.rs` (contract, `#[ignore]`, real YouTube)
25. `it_should_fetch_the_chapters_of_a_real_video` — `O728EFaXuBk` (Blender Studio, CC BY, 299 s) → its 9 chapters.
26. `it_should_download_a_video_with_its_chapters_embedded` — real 480p download reports the 9 chapters and the mp4 carries 9 matching markers (`ffprobe`).
