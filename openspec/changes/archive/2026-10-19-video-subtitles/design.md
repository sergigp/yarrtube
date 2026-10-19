# Design

Requirements: see `specs/`. Non-obvious decisions are one-line comments on
the types they shape.

## Files

- `migrations/0010_video_subtitles.sql`: `videos.subtitle_state` (`NOT NULL DEFAULT 'MISSING'`, which backfills the library), `videos.subtitle_filename`, `videos.subtitles_first_missing_at` (nullable), new `video_transcripts` table keyed by `video_id` (no FK: foreign keys are not enabled, deleters delete transcripts explicitly).
- `src/domain/video/subtitle_state.rs`: `SubtitleState` value object.
- `src/domain/video/video.rs`: subtitle fields + transitions on `Video`; `reset_for_redownload` resets them.
- `src/domain/video/episode_paths.rs`: `from_media(filename)` + `subtitle(lang)`, the one place naming `<media stem>.<lang>.srt` (by the base language code).
- `src/domain/video/events.rs`, `src/domain/event/domain_event.rs`: `VideoSubtitlesFetched` event + its `DomainEvent` variant.
- `src/domain/{playlist_video,channel_video}/events.rs`: `VideoRemovedFrom{Playlist,Channel}` carry `subtitle_filename` (the subscribers only see the event, the row is already deleted).
- `src/domain/video/video_output_entry.rs`: `subtitle_staging_dir`, a sibling of the thumbnail staging dir (the thumbnail fetch deletes its own dir recursively, possibly while a subtitle fetch is running in its lane).
- `src/domain/video_transcript/{mod,video_transcript,caption_language,caption_source,caption_track,caption_tracks,transcript_text}.rs`: new aggregate; track selection and SRT to text are pure functions here.
- `src/domain/services/subtitle_fetcher.rs`: `SubtitleFetcher`, fetches one video's subtitles and runs reconcile's missing-subtitles pass.
- `src/domain/services/internal_video_reconciler.rs`: calls `schedule_missing`; protects `subtitle_filename` in the orphan sweep, and `<media stem>.*` of Downloaded videos whose subtitles are Missing (`subtitle_pending_paths`).
- `src/domain/services/{channel,playlist}_deleter.rs`: delete each video's transcript next to `video_repository.delete`.
- `src/domain/services/{playlist,channel}_video_reconciler.rs`: delete the transcript next to `video_repository.delete`.
- `src/domain/services/video_file_deleter.rs`: deletes the subtitle file.
- `src/domain/services/task_view_searcher.rs`: resolves `fetch_subtitles` payloads like `fetch_thumbnail`.
- `src/domain/services/mod.rs`: exports.
- `src/domain/task/task.rs`: `Task::FetchSubtitles`, its payload decode, lane and `video:<id>` exclusivity key; `DeleteVideoFile.subtitle_filename`; `decode_delete_video_file_payload` returns `DeleteVideoFilePayload`.
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
- `src/infrastructure/shared/ytdlp.rs`: the two yt-dlp invocations + info-JSON parsing + probe warnings.
- `src/infrastructure/shared/ytdlp_fixtures/*.info.json`: trimmed real info-JSON fixtures.
- `src/serve.rs`, `src/infrastructure/infrastructure_container.rs`, `src/domain/mod.rs`, `src/infrastructure/repositories/mod.rs`: wiring and module exports.
- Test wiring only (new constructor arguments): `src/application/http/{channels,playlists}/mod.rs`, `src/application/subscribers/reconcile_on_{channel,playlist}_created.rs`.
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
    pub subtitles_first_missing_at: Option<DateTime<Utc>>, // first conclusive check without a selectable track; starts the 7-day window
}
impl Video {
    /// Downloaded, numbered, not in the legacy per-video-folder layout (subtitles wait for migrate-layout), and subtitle state Missing.
    pub fn is_subtitle_fetchable(&self) -> bool;
    /// Fetched + filename; stored via `update_subtitles` only.
    pub fn with_subtitles(self, subtitle_filename: impl Into<String>, now: DateTime<Utc>) -> Self;
    pub fn mark_subtitles_unavailable(self, now: DateTime<Utc>) -> Self;
    /// Records `now` as the first check without captions unless one is recorded.
    pub fn mark_subtitles_first_missing(self, now: DateTime<Utc>) -> Self;
    // reset_for_redownload (existing) also resets subtitle_state to Missing and clears subtitle_filename and subtitles_first_missing_at
}

// domain/video/episode_paths.rs
impl EpisodePaths {
    /// Folder + stem of the recorded media filename: sidecars written after the download follow the file, not the (changeable) title.
    pub fn from_media(media_filename: &str) -> Self;
    pub fn subtitle(&self, language: &CaptionLanguage) -> String; // "<stem>.<language.base()>.srt": media servers recognise base codes, not always regional ones
}

// domain/video/events.rs — published once the subtitle file is placed
pub struct VideoSubtitlesFetched { pub video_id: String, pub output_dir: String }
impl VideoSubtitlesFetched { pub const EVENT_TYPE: &str = "video_subtitles_fetched"; }

// domain/{playlist_video,channel_video}/events.rs
pub struct VideoRemovedFromPlaylist { /* ...existing */ pub subtitle_filename: Option<String> } // same on VideoRemovedFromChannel; #[serde(default)] sits on the subscribers' payload structs, where older events are decoded

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
    /// For the reported language, then its base: manual `<lang>`/`<lang>-*`, else automatic
    /// `<lang>-orig`, else plain automatic `<lang>` only when no automatic `*-orig` exists
    /// (plain codes are otherwise translations); else the only automatic `*-orig`, with its own language.
    /// No language reported: the only automatic `*-orig`, else the uploaded track when all
    /// uploaded tracks share one base language (base code preferred), else None.
    pub fn original(&self) -> Option<CaptionTrack>;
}

// domain/video_transcript/transcript_text.rs
pub struct TranscriptText(String);
impl TranscriptText {
    /// Block by block: drops each cue's index line and timing line and tag-like `<…>` markup only;
    /// keeps digit-only lines, literal `<` and `-->` in text; collapses lines repeated by rolling auto captions.
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
    pub warnings: Vec<String>, // stderr `WARNING:` lines; any warning makes an empty result inconclusive (yt-dlp withholds tracks silently, e.g. PO token)
}
pub struct FetchedSubtitles { pub path: PathBuf, pub srt: String }
/// `-J --skip-download --write-subs` without `--no-warnings` (subtitle warnings only appear when subtitles are requested; -J writes no files);
/// Err on any failure (clean or systemic), so the caller leaves the video Missing.
pub fn probe_captions(ytdlp_path: &Path, video_url: &str, staging_dir: &Path) -> Result<CaptionProbe>;
/// `--load-info-json --skip-download --write-subs|--write-auto-subs --sub-langs <key> --sub-format vtt --convert-subs srt`.
/// Returns the staged `<id>.<track.key>.srt` (not any .srt: a kill can leave another track's file behind).
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

// domain/services/{channel,playlist}_deleter.rs — new third constructor argument
// video_transcript_repository: Arc<dyn VideoTranscriptRepository>

// infrastructure/repositories/sqlite_video_repository.rs
trait VideoRepository {
    // ...existing
    /// Writes only subtitle_state, subtitle_filename, subtitles_first_missing_at, updated_at.
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
impl Task {
    pub fn decode_fetch_subtitles_payload(payload: &str) -> Result<(String, String), TaskError>;
    pub fn decode_delete_video_file_payload(payload: &str) -> Result<DeleteVideoFilePayload, TaskError>;
}
// "fetch_subtitles" exclusivity key: video:<id>, shared with download_video and fetch_thumbnail
pub struct DeleteVideoFilePayload { pub filename: Option<String>, pub thumbnail_filename: Option<String>, #[serde(default)] pub subtitle_filename: Option<String>, pub output_dir: String } // serde(default): tasks queued before upgrade still decode

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
        None, !probe.warnings.is_empty() => warn(warnings), leave Missing   // inconclusive
        None =>
          video = video.mark_subtitles_first_missing(now)          // keeps an earlier time
          published_at.unwrap_or(video.created_at) <= now - UNAVAILABLE_AFTER
            && video.subtitles_first_missing_at <= now - UNAVAILABLE_AFTER
            ? VideoRepository::update_subtitles(&video.mark_subtitles_unavailable(now))
            : VideoRepository::update_subtitles(&video)            // Missing, first check recorded
        Some(track) =>
          VideoDownloaderRepository::fetch_subtitles(&probe, &track, staging) -> Err => warn, leave Missing
          VideoRepository::find(&video.id) -> None, or no longer Downloaded with the same filename => return   // deleted or reset meanwhile, like VideoDownloader::record_unless_deleted
          path = EpisodePaths::from_media(&video.filename).subtitle(&track.language)
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
    delete_orphaned_files(&actual)   // protected_entries includes every subtitle_filename; subtitle_pending_paths protects "<media stem>."* of Downloaded videos whose subtitles are Missing
    (redownload_broken_files: Video::reset_for_redownload also resets the subtitle fields)
```

Removal:
```
{Playlist,Channel}VideoReconciler publishes VideoRemovedFrom{Playlist,Channel}{ ..., subtitle_filename: video.subtitle_filename }
delete_video_file_on_video_removed_from_{playlist,channel}
  TaskRepository::schedule(&Task::DeleteVideoFile{ filename, thumbnail_filename, subtitle_filename: event.subtitle_filename, output_dir })
DeleteVideoFileTask::handle -> VideoFileDeleter::delete_video_file(filename, thumbnail, subtitle, output_dir)

Channel/playlist deleted (HTTP):
{Channel,Playlist}Deleter::delete -> for each video: VideoRepository::delete(id); VideoTranscriptRepository::delete(id)
  publishes {Channel,Playlist}Deleted -> Delete{Channel,Playlist}FilesTask removes the show directory (subtitle files included)
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
26. `caption_tracks.rs`: manual exact, manual regional (`en-US`), `-orig` fallback, plain automatic fallback (only when no `*-orig` exists), translations only -> None, no language + single `-orig`, no language + no `-orig` + no manual -> None.
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

Audit follow-up (already implemented):

`application/tasks/fetch_subtitles_task.rs`
46. `it_should_mark_a_video_published_exactly_seven_days_ago_unavailable` (boundary).
47. `it_should_not_publish_that_subtitles_were_fetched_if_unavailable` / `..._if_the_fetch_fails`.
48. `it_should_discard_the_subtitles_if_the_video_was_deleted_meanwhile`.
49. `it_should_name_the_subtitles_after_the_downloaded_file`: filename `... - Old Title.mp4`, title `New Title` -> `... - Old Title.en.srt`.

`application/tasks/reconcile_channel_task.rs`
50. `it_should_keep_subtitle_files_placed_but_not_yet_recorded`.
51. `it_should_keep_unrecorded_subtitle_files_of_a_retitled_video`.
52. `it_should_clear_the_subtitles_of_a_video_redownloaded`.

`application/http/{channels,playlists}/mod.rs`
53. `it_should_delete_the_transcripts_of_the_deleted_{channel,playlist}_videos`.

`domain/video/video.rs`, `domain/video/episode_paths.rs`
54. `it_should_record_its_fetched_subtitles`, `it_should_mark_its_subtitles_unavailable`, `it_should_clear_its_subtitles_when_reset_for_redownload`, `it_should_name_an_episodes_files_after_its_downloaded_media_file`.

Audit follow-up (to implement):

`application/tasks/fetch_subtitles_task.rs`
55. `it_should_keep_an_old_video_checked_for_the_first_time_missing`: published 30d ago, no first check -> Missing, `subtitles_first_missing_at = now`.
56. `it_should_mark_unavailable_seven_days_after_the_first_check_without_captions`: first check 8d ago -> Unavailable; first check 2d ago -> Missing, time unchanged.
57. `it_should_keep_subtitles_missing_when_the_downloader_warns`: probe with a warning and no selectable track -> Missing, no first check recorded.
58. `it_should_clear_the_first_check_without_captions_when_reset_for_redownload` (`video.rs` unit).

`caption_tracks.rs`
59. `it_should_select_the_only_original_when_the_reported_language_has_none`: `en` reported, automatic `{es-orig, es, en, fr}` -> `es-orig`, language `es` (fixture `auto_only_spanish.info.json` shape).
60. `it_should_not_select_a_plain_track_when_an_original_exists_in_another_language` (covered by 59) and `it_should_select_a_plain_track_when_no_original_is_marked` (`yue` only).
61. `it_should_select_the_uploaded_track_when_no_language_is_reported`: manual `{en, en-GB}` -> `en`; manual `{en, de}` -> None.

`transcript_text.rs`
62. `it_should_keep_a_text_line_made_of_digits`, `it_should_keep_a_literal_less_than_sign`, `it_should_keep_text_containing_an_arrow`; existing index/timing/tag/rolling tests keep passing.

`infrastructure/shared/ytdlp.rs`
63. `it_should_request_subtitles_and_keep_warnings_when_probing`: args include `--write-subs`, exclude `--no-warnings`.
64. `it_should_collect_the_warnings_of_a_probe`: fake yt-dlp prints `WARNING: ...` on stderr, exit 0 -> `probe.warnings` holds it.

`infrastructure/repositories/sqlite_video_repository.rs`
65. `it_should_round_trip_the_first_check_without_captions` (and `update_subtitles` writes it).

Re-audit follow-up:

66. `video.rs`: a Downloaded numbered video in the legacy layout is not subtitle-fetchable; `fetch_subtitles_task.rs`: legacy video -> no downloader call, stays Missing; `reconcile_channel_task.rs`: no FetchSubtitles scheduled for it.
67. `episode_paths.rs`: `from_media` of a bare filename yields a relative path (no leading `/`).
68. `fetch_subtitles_task.rs`: video reset for redownload while fetching -> nothing placed, no transcript, no event, stays Missing.
69. `ytdlp.rs`: a stale `<id>.es.srt` in staging is ignored when fetching key `en`.

