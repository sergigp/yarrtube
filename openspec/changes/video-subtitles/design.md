# Design

Requirements: see `specs/`. Non-obvious decisions are one-line comments on
the types they shape.

## Files

- `migrations/0010_video_subtitles.sql`: `videos.subtitle_state` (`NOT NULL DEFAULT 'MISSING'`, which backfills the library), `videos.subtitle_filename`, new `video_transcripts` table keyed by `video_id`.
- `src/domain/video/subtitle_state.rs`: `SubtitleState` value object.
- `src/domain/video/video.rs`: subtitle fields + transitions on `Video`.
- `src/domain/video/episode_paths.rs`: `subtitle(lang)`, the one place naming `<stem>.<lang>.srt` (by the base language code).
- `src/domain/video/events.rs`, `src/domain/event/domain_event.rs`: `VideoSubtitlesFetched` event + its `DomainEvent` variant.
- `src/domain/{playlist_video,channel_video}/events.rs`: `VideoRemovedFrom{Playlist,Channel}` carry `subtitle_filename` (the subscribers only see the event, the row is already deleted).
- `src/domain/video/video_output_entry.rs`: `subtitle_staging_dir`, a sibling of the thumbnail staging dir (the thumbnail fetch deletes its own dir recursively, possibly while a subtitle fetch is running in its lane).
- `src/domain/video_transcript/{mod,video_transcript,caption_language,caption_source,caption_track,caption_tracks,transcript_text}.rs`: new aggregate; track selection and SRT to text are pure functions here.
- `src/domain/services/subtitle_fetcher.rs`: `SubtitleFetcher`, fetches one video's subtitles and runs reconcile's missing-subtitles pass.
- `src/domain/services/internal_video_reconciler.rs`: calls `schedule_missing`; protects `subtitle_filename` in the orphan sweep.
- `src/domain/services/{playlist,channel}_video_reconciler.rs`: delete the transcript next to `video_repository.delete`.
- `src/domain/services/video_file_deleter.rs`: deletes the subtitle file.
- `src/domain/services/task_view_searcher.rs`: resolves `fetch_subtitles` payloads like `fetch_thumbnail`.
- `src/domain/services/mod.rs`: exports.
- `src/domain/task/task.rs`: `Task::FetchSubtitles`, its payload decode and lane; `DeleteVideoFile.subtitle_filename`.
- `src/domain/task/task_lane.rs`: `TaskLane::Subtitles` (the executor's `_ => 1` already caps it at 1).
- `src/application/tasks/fetch_subtitles_task.rs`: task handler.
- `src/application/tasks/mod.rs`: registers it.
- `src/application/subscribers/delete_video_file_on_video_removed_from_{playlist,channel}.rs`: pass `subtitle_filename`.
- `src/application/subscribers/fetch_subtitles_on_video_downloaded.rs`: schedules `FetchSubtitles` on `VideoDownloaded`, so subtitles don't wait for the next reconcile.
- `src/application/subscribers/scan_plex_folder_on_video_subtitles_fetched.rs`: Plex scan on `VideoSubtitlesFetched`, mirrors `scan_plex_folder_on_video_downloaded.rs`.
- `src/application/subscribers/mod.rs`: registers both (the Plex one only when configured).
- `src/infrastructure/repositories/sqlite_video_transcript_repository.rs`: `VideoTranscriptRepository` port + SQLite implementation.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: map new columns; `update_subtitles`.
- `src/infrastructure/shared/sqlite_migrations.rs`: migration `0010` default test.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`: `probe_captions`, `fetch_subtitles` on the port, the yt-dlp implementation and the fake.
- `src/infrastructure/shared/ytdlp.rs`: the two yt-dlp invocations + info-JSON parsing.
- `src/serve.rs`: wiring.
- `web/src/lib/tasks.ts` (+ `tasks.test.ts`): describe/family/kind for `fetch_subtitles`.
- `doc/ARCHITECTURE.md`, `doc/PLEX.md`: mention subtitle files, the fetch after download, the Plex rescan and the backfill; the ARCHITECTURE example walks through the subtitle flow (steps + sequence diagram).

## Types & Signatures

```rust
// domain/video/subtitle_state.rs — stored as MISSING / FETCHED / UNAVAILABLE
pub enum SubtitleState { Missing, Fetched, Unavailable }
impl SubtitleState {
    pub fn as_str(&self) -> &'static str;
    pub fn parse(value: &str) -> Result<Self, ValidationError>;
}

// domain/video/video.rs
pub struct Video {
    // ...existing fields
    pub subtitle_state: SubtitleState,
    pub subtitle_filename: Option<String>,
}
impl Video {
    /// Downloaded, numbered, and subtitle state Missing.
    pub fn is_subtitle_fetchable(&self) -> bool;
    /// Fetched + filename; stored via `update_subtitles` only.
    pub fn with_subtitles(self, subtitle_filename: impl Into<String>, now: DateTime<Utc>) -> Self;
    pub fn mark_subtitles_unavailable(self, now: DateTime<Utc>) -> Self;
}

// domain/video/episode_paths.rs
impl EpisodePaths { pub fn subtitle(&self, language: &CaptionLanguage) -> String; } // "<stem>.<language.base()>.srt": media servers recognise base codes, not always regional ones

// domain/video/events.rs — published once the subtitle file is placed
pub struct VideoSubtitlesFetched { pub video_id: String, pub output_dir: String }
impl VideoSubtitlesFetched { pub const EVENT_TYPE: &str = "video_subtitles_fetched"; }

// domain/{playlist_video,channel_video}/events.rs
pub struct VideoRemovedFromPlaylist { /* ...existing */ #[serde(default)] pub subtitle_filename: Option<String> } // same on VideoRemovedFromChannel

// domain/video/video_output_entry.rs
pub fn subtitle_staging_dir(videos_path: &str, video_id: &VideoRecordId) -> PathBuf; // <staging>/<id>-subtitles

// domain/video_transcript/caption_language.rs — e.g. "en", "pt-BR"; never carries "-orig"
pub struct CaptionLanguage(String);
impl CaptionLanguage {
    pub fn new(value: &str) -> Result<Self, ValidationError>;
    pub fn as_str(&self) -> &str;
    pub fn base(&self) -> &str; // primary subtag: "es-US" -> "es", "en" -> "en"
}

// domain/video_transcript/caption_source.rs — stored as MANUAL / AUTOMATIC
pub enum CaptionSource { Manual, Automatic }

// domain/video_transcript/caption_track.rs
pub struct CaptionTrack {
    pub key: String,               // yt-dlp's --sub-langs key, e.g. "en-orig"
    pub language: CaptionLanguage, // "en"
    pub source: CaptionSource,
}

// domain/video_transcript/caption_tracks.rs — what yt-dlp's info JSON lists
pub struct CaptionTracks {
    pub original_language: Option<String>, // info JSON `language`
    pub manual: Vec<String>,               // keys of `subtitles`
    pub automatic: Vec<String>,            // keys of `automatic_captions`
}
impl CaptionTracks {
    /// Manual `<lang>`/`<lang>-*`, else automatic `<lang>-orig`, else
    /// automatic `<lang>`; no language reported: the only automatic `*-orig`.
    pub fn original(&self) -> Option<CaptionTrack>;
}

// domain/video_transcript/transcript_text.rs
pub struct TranscriptText(String);
impl TranscriptText {
    /// Drops indices, timings and tags; collapses lines repeated by rolling auto captions.
    pub fn from_srt(srt: &str) -> Self;
    pub fn as_str(&self) -> &str;
}

// domain/video_transcript/video_transcript.rs
pub struct VideoTranscript {
    pub video_id: VideoRecordId,
    pub language: CaptionLanguage,
    pub source: CaptionSource,
    pub text: TranscriptText,
    pub created_at: DateTime<Utc>,
}
impl VideoTranscript {
    pub fn create(video_id: VideoRecordId, track: &CaptionTrack, text: TranscriptText, now: DateTime<Utc>) -> Self;
}

// infrastructure/shared/ytdlp.rs
pub struct CaptionProbe {
    pub tracks: CaptionTracks,
    pub published_at: Option<DateTime<Utc>>,
    pub info_json: PathBuf, // reused by fetch_subtitles (--load-info-json): one extraction per video, fewer YouTube requests
}
pub struct FetchedSubtitles { pub path: PathBuf, pub srt: String }
/// `-J --skip-download`; Err on any failure (clean or systemic), so the caller leaves the video Missing.
pub fn probe_captions(ytdlp_path: &Path, video_url: &str, staging_dir: &Path) -> Result<CaptionProbe>;
/// `--load-info-json --skip-download --write-subs|--write-auto-subs --sub-langs <key> --sub-format vtt --convert-subs srt`.
pub fn fetch_subtitles(ytdlp_path: &Path, probe: &CaptionProbe, track: &CaptionTrack, staging_dir: &Path) -> Result<FetchedSubtitles>;

// infrastructure/repositories/youtube_video_downloader_repository.rs
pub trait VideoDownloaderRepository: Send + Sync {
    // ...existing
    fn probe_captions(&self, video_url: &str, staging_dir: &Path) -> anyhow::Result<CaptionProbe>;
    fn fetch_subtitles(&self, probe: &CaptionProbe, track: &CaptionTrack, staging_dir: &Path) -> anyhow::Result<FetchedSubtitles>;
}
impl FakeVideoDownloaderRepository {
    pub fn with_caption_probe(self, probe: anyhow::Result<CaptionProbe>) -> Self;
    pub fn with_subtitles_result(self, result: anyhow::Result<FetchedSubtitles>) -> Self;
    // + caption_calls: Mutex<usize>
}

// infrastructure/repositories/sqlite_video_transcript_repository.rs
pub trait VideoTranscriptRepository: Send + Sync {
    fn save(&self, transcript: &VideoTranscript) -> anyhow::Result<()>; // upsert by video_id
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoTranscript>>; // #[allow(dead_code)] until a feature reads transcripts
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}
pub struct SqliteVideoTranscriptRepository { db: Database }

// infrastructure/repositories/sqlite_video_repository.rs
trait VideoRepository {
    // ...existing
    /// Writes only subtitle_state, subtitle_filename, updated_at.
    fn update_subtitles(&self, video: &Video) -> anyhow::Result<()>;
}

// domain/services/subtitle_fetcher.rs
const UNAVAILABLE_AFTER: Duration = Duration::days(7);
pub struct SubtitleFetcher {
    video_repository: Arc<dyn VideoRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    task_repository: Arc<dyn TaskRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    clock: Arc<dyn Clock>,
    videos_path: String,
}
pub trait SubtitleFetcherApi: Send + Sync {
    /// No-op unless `is_subtitle_fetchable`. Best effort: logs and swallows every failure.
    /// Publishes `VideoSubtitlesFetched` after recording Fetched.
    fn fetch(&self, video: &Video, output_dir: &Path);
    /// Schedules FetchSubtitles for each fetchable video not in `skip_ids`.
    fn schedule_missing(&self, videos: &[Video], skip_ids: &HashSet<&VideoRecordId>, output_dir: &Path) -> anyhow::Result<()>;
}

// domain/task/task.rs
pub enum Task {
    // ...
    DeleteVideoFile { filename: Option<String>, thumbnail_filename: Option<String>, subtitle_filename: Option<String>, output_dir: String },
    FetchSubtitles { video_id: String, output_dir: String }, // "fetch_subtitles" -> TaskLane::Subtitles
}
impl Task { pub fn decode_fetch_subtitles_payload(payload: &str) -> Result<(String, String), TaskError>; }
// DeleteVideoFilePayload.subtitle_filename: Option<String> with #[serde(default)]: tasks queued before upgrade still decode.

// domain/task/task_lane.rs
pub enum TaskLane { Download, Thumbnail, Subtitles, Light, Exclusive }

// domain/services/video_file_deleter.rs
fn delete_video_file(&self, filename: Option<String>, thumbnail_filename: Option<String>, subtitle_filename: Option<String>, output_dir: &Path) -> anyhow::Result<()>;

// application/tasks/fetch_subtitles_task.rs
pub struct FetchSubtitlesTask { video_repository: Arc<dyn VideoRepository>, subtitle_fetcher: Arc<SubtitleFetcher> }
impl TaskHandler for FetchSubtitlesTask { fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()>; } // always Ok

// application/subscribers/fetch_subtitles_on_video_downloaded.rs — reads video_id + output_dir from the payload, no lookup
pub struct FetchSubtitlesOnVideoDownloaded { task_repository: Arc<dyn TaskRepository>, clock: Arc<dyn Clock> }
impl EventSubscriber for FetchSubtitlesOnVideoDownloaded { fn handle(&self, payload: &str) -> anyhow::Result<()>; }

// application/subscribers/scan_plex_folder_on_video_subtitles_fetched.rs — registered only when Plex is configured
pub struct ScanPlexFolderOnVideoSubtitlesFetched { scanner: PlexFolderScanner }
impl EventSubscriber for ScanPlexFolderOnVideoSubtitlesFetched { fn handle(&self, payload: &str) -> anyhow::Result<()>; } // Err retries the event
```

## Call Stack

Download finished:
```
VideoDownloader publishes VideoDownloaded{video_id, output_dir, folder}
  FetchSubtitlesOnVideoDownloaded::handle(payload)
    TaskRepository::schedule(&Task::FetchSubtitles{video_id, output_dir}, now)   // dedupes queued
  ScanPlexFolderOnVideoDownloaded::handle(payload)                                // existing
```

Subtitle fetch:
```
TaskExecutor (Subtitles lane)
  FetchSubtitlesTask::handle(payload)
    Task::decode_fetch_subtitles_payload(payload) -> (video_id, output_dir)
    VideoRepository::find(&video_id) -> None => Ok(())
    SubtitleFetcher::fetch(&video, output_dir)
      video.is_subtitle_fetchable() -> false => return
      VideoDownloaderRepository::probe_captions(url, subtitle_staging_dir) -> Err => warn, leave Missing
      probe.tracks.original()
        None => published_at.unwrap_or(video.created_at) <= now - UNAVAILABLE_AFTER
                  ? VideoRepository::update_subtitles(&video.mark_subtitles_unavailable(now))
                  : leave Missing
        Some(track) =>
          VideoDownloaderRepository::fetch_subtitles(&probe, &track, staging) -> Err => warn, leave Missing
          path = EpisodePaths::new(&number, &video.title).subtitle(&track.language)
          VideoFileRepository::move_file(&fetched.path, &output_dir.join(&path))
          VideoTranscriptRepository::save(&VideoTranscript::create(id, &track, TranscriptText::from_srt(&fetched.srt), now))
          VideoRepository::update_subtitles(&video.with_subtitles(path, now))
          EventPublisher::publish(&DomainEvent::VideoSubtitlesFetched{video_id, output_dir}) -> Err => warn
      VideoFileRepository::delete_dir_recursive(subtitle_staging_dir)   // always

ScanPlexFolderOnVideoSubtitlesFetched::handle(payload)
  PlexFolderScanner::scan_folder(output_dir)
```

Reconcile:
```
Reconcile{Playlist,Channel}Task -> {Playlist,Channel}VideoReconciler::reconcile
  on removed video: VideoRepository::delete(id); VideoTranscriptRepository::delete(id)
  InternalVideoReconciler::reconcile(desired, delta)
    ... existing steps; handled = added + redownloaded + recovered
    ThumbnailFetcher::schedule_missing(&actual.videos, &handled, &output_dir)
    SubtitleFetcher::schedule_missing(&actual.videos, &handled, &output_dir)
      filter is_subtitle_fetchable && !handled -> TaskRepository::schedule(&Task::FetchSubtitles{..}, now) // dedupes queued
    delete_orphaned_files(&actual)   // protected_entries includes every subtitle_filename
```

Removal:
```
{Playlist,Channel}VideoReconciler publishes VideoRemovedFrom{Playlist,Channel}{ ..., subtitle_filename: video.subtitle_filename }
delete_video_file_on_video_removed_from_{playlist,channel}
  TaskRepository::schedule(&Task::DeleteVideoFile{ filename, thumbnail_filename, subtitle_filename: event.subtitle_filename, output_dir })
DeleteVideoFileTask::handle -> VideoFileDeleter::delete_video_file(filename, thumbnail, subtitle, output_dir)
```

## Test Plan

Behaviour (acceptance through the adapters):

`application/tasks/fetch_subtitles_task.rs`
1. `it_should_fetch_the_original_language_subtitles`: srt moved to `Season 2026/S2026E031501 - Title.en.srt`; video Fetched with that filename; transcript stored (en, Manual, plain text).
2. `it_should_record_auto_generated_captions_as_automatic`: probe with only `es-orig` automatic -> transcript `es`, Automatic; file `.es.srt`.
3. `it_should_mark_an_old_video_without_captions_unavailable`: no selectable track, published 30 days ago -> Unavailable, no file, no transcript.
4. `it_should_keep_a_recent_video_without_captions_missing`: published 2 days ago -> still Missing.
5. `it_should_fall_back_to_the_creation_time_when_no_publish_time_is_reported`: probe `published_at: None`, created 30 days ago -> Unavailable.
6. `it_should_leave_subtitles_missing_when_the_probe_fails`: probe Err -> Missing, no transcript, task Ok.
7. `it_should_leave_subtitles_missing_when_the_download_fails`: fetch Err -> Missing, no file moved.
8. `it_should_skip_if_subtitles_already_fetched`: Fetched/Unavailable video -> no downloader call.
9. `it_should_skip_a_video_that_is_not_downloaded`: Pending video -> no downloader call.
10. `it_should_skip_if_video_no_longer_exists`.
11. `it_should_keep_download_progress_while_recording_subtitles`: video updated between read and record -> download fields kept.
12. `it_should_discard_the_staging_dir`: staging dir deleted on success and on failure.

`application/tasks/reconcile_channel_task.rs` (primary for `InternalVideoReconciler`)
13. `it_should_schedule_a_subtitle_fetch_for_a_downloaded_video_missing_subtitles`.
14. `it_should_not_schedule_a_subtitle_fetch_for_fetched_or_unavailable_subtitles`.
15. `it_should_not_schedule_a_subtitle_fetch_for_a_video_not_downloaded`.
16. `it_should_not_schedule_a_second_subtitle_fetch_if_one_is_queued`.
17. `it_should_not_schedule_a_subtitle_fetch_for_a_video_being_redownloaded`.
18. `it_should_keep_recorded_subtitle_files`: orphan sweep leaves `<stem>.en.srt`; a stray `.srt` is deleted.
19. `it_should_delete_the_transcript_of_a_removed_video`.

`application/tasks/reconcile_playlist_task.rs`
20. `it_should_schedule_a_subtitle_fetch_for_a_downloaded_video_missing_subtitles` (wiring happy path).
21. `it_should_delete_the_transcript_of_a_removed_video`.

`application/subscribers/delete_video_file_on_video_removed_from_{playlist,channel}.rs`
22. `it_should_schedule_the_deletion_of_the_subtitle_file` (each subscriber).

`application/tasks/delete_video_file_task.rs`
23. `it_should_delete_an_episodes_subtitle_file`.
24. `it_should_decode_a_payload_queued_without_a_subtitle_filename`.

`application/http/tasks/mod.rs`
25. `it_should_list_a_fetch_subtitles_task_with_video_and_playlist_names`.

Value objects / pure domain (own files):
26. `caption_tracks.rs`: manual exact, manual regional (`en-US`), `-orig` fallback, plain automatic fallback, translations only -> None, no language + single `-orig`, no language + no `-orig` -> None.
27. `transcript_text.rs`: strips indices/timings/tags; collapses rolling duplicates; empty input.
28. `subtitle_state.rs`, `caption_language.rs` (rejects empty, strips nothing; `-orig` rejected), `episode_paths.rs` `subtitle`.

`web/src/lib/tasks.test.ts`
29. describes `fetch_subtitles` with and without resolved names; family `downloads`; kind `download`.

Follow-up (fetch after download, Plex rescan, base language code):

`application/subscribers/fetch_subtitles_on_video_downloaded.rs`
40. `it_should_schedule_a_subtitle_fetch_when_a_video_is_downloaded`: `FetchSubtitles{video_id, output_dir}` pending, run now.
41. `it_should_not_schedule_a_second_subtitle_fetch_if_one_is_queued`.

`application/tasks/fetch_subtitles_task.rs`
42. `it_should_publish_that_subtitles_were_fetched`: outbox holds `VideoSubtitlesFetched{video_id, output_dir}`; none when Unavailable or failed.
43. `it_should_name_a_regional_track_with_its_base_language`: reported `es-US`, manual `es-US` -> `.es.srt`; transcript language `es-US`.

`application/subscribers/scan_plex_folder_on_video_subtitles_fetched.rs`
44. `it_should_scan_the_show_folder_after_subtitles_are_fetched`: scan on every containing section.

`caption_language.rs`
45. `it_should_return_the_base_language`: `es-US` -> `es`, `en` -> `en`.

Infrastructure:

`infrastructure/shared/ytdlp.rs` (fake yt-dlp scripts, like existing tests; info-JSON fixtures captured from real yt-dlp in task 3.1)
30. `it_should_parse_caption_tracks_and_language_from_the_info_json`.
31. `it_should_parse_the_publish_timestamp_from_the_info_json`.
32. `it_should_fail_the_probe_on_a_clean_failed_exit`.
33. `it_should_pass_manual_subtitle_flags_for_a_manual_track` / `..._auto_subtitle_flags_for_an_automatic_track`.
34. `it_should_return_the_written_srt_and_its_contents`.
35. `it_should_fail_when_no_srt_was_written`.

`infrastructure/repositories/sqlite_video_transcript_repository.rs`
36. `it_should_replace_the_transcript_of_a_video_on_save`.
37. `it_should_delete_a_video_transcript`.

`infrastructure/repositories/sqlite_video_repository.rs`
38. `it_should_update_only_the_subtitle_columns`.

`infrastructure/shared/sqlite_migrations.rs`
39. `it_should_default_existing_videos_to_missing_subtitles` (migration).
