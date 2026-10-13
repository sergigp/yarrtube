## Context

See proposal.md for why. Current state that shapes the approach:

- A download resolves a per-video folder up front (`ytdlp::prepare_video_dir`,
  collision-suffixed with the YouTube id) and runs `yt-dlp` inside it with
  `-o "<folder>.%(ext)s"`. The thumbnail-ahead fetch does the same with
  `--skip-download`. `movie.nfo` is written into that folder before the media.
- `videos.filename` / `videos.thumbnail_filename` are stored relative to the
  source's output directory; the SPA builds `/media/<path>/<filename>` from
  them. There is no publish timestamp on `videos`; it lives on
  `video_metadata.published_at` (Data API, best effort).
- The reconciler (`InternalVideoReconciler`) sweeps the source directory's
  top-level entries, protecting recorded entries plus "folders a download may
  be writing into" derived from titles.
- Plex: `PlexFolderScanner` (sections from env) is already driven by the
  `VideoDownloaded` event; collections are a separate recurring task and two
  deletion subscribers, all over `PlexCollectionRepository`.
- Episode identity in Plex's NFO Series agent is show + season + episode, so
  an episode number must never change once files exist.
- A playlist is a curated sequence whose order upload dates do not follow
  (a checked kids' show playlist: 83 of 154 "next episode" steps would be
  wrong in upload order), so playlists cannot share the channels' date rule.
- Video rows are per source: each playlist or channel link creates its own
  `Video` row, so a row belongs to exactly one show. `playlist_videos`
  already stores each video's YouTube `position`.

## Goals / Non-Goals

**Goals:**
- Files appear in the show folder only in their final name, next to their NFO.
- Two numbering rules (channel: publish date; playlist: playlist order),
  each assigned once, stored on the video, never recomputed or reused.
- The migration and the reconcile recovery share one "relocate a video" path.
- Plex code shrinks to the scanner.

**Non-Goals:**
- Show-level genres (Categories tab), playlist posters, 2:3 poster
  generation, Plex playlists, copying Plex watch history.
- Keeping the previous layout alive beyond "still playable until migrated".

## Decisions

1. **Stage downloads outside the show, move into place.** `yt-dlp` writes
   into `<videos root>/.downloads/<video id>/` with `-o "%(id)s.%(ext)s"` and
   prints `%(timestamp)s` / `%(upload_date)s`. The downloader then assigns the
   number, writes the NFO at its final path and renames media + thumbnail
   into `Season <year>/`. A rename within the same mount is atomic and cheap;
   Plex and Kodi ignore dot-prefixed directories, so partial files, `.part`
   and `.fNNN` intermediates never reach the library, and the reconciler no
   longer needs title-derived "in-flight folder" protection. The same
   staging serves the thumbnail-ahead fetch. The staging dir is keyed by the
   video's record id, not its YouTube id: the same YouTube video tracked by
   two shows is two records that may download at once, and each discards
   only its own dir. A thumbnail fetch discards its staging dir after
   success as well as failure.
2. **Publish timestamp from yt-dlp, not the Data API.** Both yt-dlp
   invocations already run per video; the Data API is best effort and
   quota-bound. The migration, which runs no yt-dlp, uses
   `video_metadata.published_at` (same UTC instant).
3. **Same-day index assigned in the database.** `VideoRepository::
   assign_episode_number` is one write through the serialized writer:
   if the video has a number, return it; else next free index among the
   videos linked to the same channel with the same season and month-day.
   No filesystem probing, no race between concurrent downloads. A playlist
   video always has its number already (decision 9), so for it this just
   returns the recorded one; an unnumbered video in no channel fails instead
   of getting a date-based number, so a playlist video can never land in
   `Season <year>`. The value rules (first of day, `+1`, index `01`..`99`)
   live in `ChannelEpisodeNumber`; the repository only does the atomic "max
   then write". A deliberate deviation from "no logic in repositories"
   (listed in rust-architect): a find-then-update race would record
   permanent duplicate numbers.
4. **One relocation service.** `VideoRelocator::relocate` (assign number from
   a given timestamp, move file set, rewrite NFO, record filenames) is used
   by the `migrate-layout` subcommand and by missing-metadata recovery for
   videos the migration skipped.
5. **Show files written by the source reconcilers.** `ShowFileWriter` runs at
   the start of every channel/playlist reconcile (initial and recurring), so
   creation, rename, avatar change and hand deletion all converge without a
   new event. Best effort: a write failure is logged and the pass carries on
   (the next pass retries); the migrator records it in `failed` under the
   source dir and still moves that source's videos.
   `FilesystemShowFileRepository` leaves `tvshow.nfo` / `poster.jpg`
   untouched when their contents are unchanged, so passes don't wake the
   disk or bump mtimes.
6. **Legacy detection by path shape.** A recorded filename is new-layout
   only when its first segment is `Season <digits>` and its file name starts
   with an `S<digits>E<digits> - ` episode code; anything else is legacy
   (per-video folder or flat file), including a per-video folder whose title
   starts with `Season ` (e.g. `Season 2 Finale/`), and keeps today's
   top-level-entry handling for delete and sweep.
7. **`VideoDownloaded` keeps its shape**; the scanner scans `output_dir` (the
   show) and ignores `folder`.
8. **Dry run plans numbers in memory.** `assign_episode_number` is a write,
   so a dry run uses `VideoRepository::next_episode_number`, a read-only
   peek with the same rule (recorded number, else next free same-day index),
   and numbers each video as `max(peek, last number planned this run for the
   same show, season and day + 1)`. Its plan equals the real run's, a
   same-day batch included, without any write. Playlist videos carry their
   recorded number, so the dry run plans them as is.
   The plan needs the current schema (on a database from before this change,
   `0008` is what numbers the playlist videos), so a dry run applies pending
   migrations to the real database first, exactly as `serve` does on its
   next start; beyond that it writes nothing.
9. **Playlist videos are numbered when first stored, from a per-playlist
   counter.** `Playlist` gains `last_episode` (highest number ever
   assigned). `PlaylistVideoReconciler::add_video` takes
   `playlist.next_episode_number()`, creates the video with that number,
   saves it, and records the counter with `PlaylistRepository::
   record_last_episode(id, number)`, which raises it (`MAX`) and never
   lowers it; `PlaylistRepository::update` does not write `last_episode`, so
   a full-row write from a stale read (the home-visibility toggle) can't move
   it back. A pass adds
   its new videos in YouTube position order, so a first pass numbers the
   playlist in its order and later additions append. The counter, not
   `MAX(episode)`, guarantees a removed highest number is never reused (Plex
   would give the new video the old one's watch state). Assigning at link
   time, not at download, keeps download order, retries and concurrency out
   of the numbering; a never-downloaded video just leaves a gap.
10. **`EpisodeNumber` is an enum of two value objects.**
    `EpisodeNumber::{Channel(ChannelEpisodeNumber), Playlist(PlaylistEpisodeNumber)}`
    is what `Video` stores and every layout helper renders (`season()`,
    `episode()`, `season_folder()`, `code()`): `S2026E031501` for a channel,
    `S01E007` (season two digits, episode at least three) for a playlist.
    The database keeps the same `season`/`episode` columns; a row decodes as
    `Playlist` when its season is `1` (a channel season is a publish year).
11. **Existing playlist videos are numbered by the database migration.**
    `0008_episode_numbers.sql` (unreleased, edited in place) also adds
    `playlists.last_episode` and numbers every video already linked to a
    playlist, any status, `ROW_NUMBER() OVER (PARTITION BY playlist_id ORDER
    BY position, id)` as season 1, then sets each playlist's counter to its
    count. So pending playlist videos download under the right number even
    before `migrate-layout` runs, and the migration never needs a publish
    time for a playlist video: one without metadata is moved, not skipped.
    It also deletes queued `reconcile_plex_collections` task rows, whose
    handler is gone.
12. **`EpisodePaths` names every episode file.** Stem
    `Season <s>/<code> - <title>`; `media(ext)`, `thumbnail()`, `nfo()`, and
    `thumb()` (the thumbnail's name as the NFO references it). The
    downloader, relocator, thumbnail fetcher and orphan sweep all build from
    it, so they can't drift.

## Risks / Trade-offs

- [Plex rejects six-digit episode numbers] → verify on a real server before
  implementation (spike task 0); the fallback is a four-digit `MMDD` episode
  with the same-day index appended to the title part of the base name,
  decided only if the spike fails.
  Spike outcome (2026-10-10, real server, TV library with
  `tv.plex.agents.nfo.series` + "Plex TV Series" scanner): six-digit
  (`S2026E031501` → episode 31501) and eight-digit (`S2026E03150201` →
  3150201) numbers both scan, with the NFO title, `aired` date, show poster.
  Renaming a watched episode's three files to `S2026E031502` and rescanning
  creates a new item (new ratingKey) that keeps the watched flag; while its
  nfo still said `<episode>31501` Plex showed "Episode 31502", so a
  renumber must rewrite the nfo's `<episode>` with the files. Six digits
  kept, no fallback needed.
- [`.downloads` staging on a different filesystem than the show dir] → both
  are under the videos root; documented as a requirement of the volume.
- [Migration interrupted mid-video] → each video is moved file by file and
  its row updated last; a rerun sees either the legacy path (redo) or the
  new path (skip). A half-moved set (media moved, row not updated) is
  detected by the media existing at the target and is completed, not
  duplicated: the number was recorded before any move, so the target is
  known (`is_half_moved`), and each move skips a file already at its target
  (`already_moved`).
- [Media move fails after the number is assigned] → the number stays
  recorded while paths and status are unchanged; a rerun reuses it, so
  numbering stays stable.
- [Two copies of one YouTube video in one show] → impossible, a source holds
  a video once; copies across sources number independently.
- [Playlist reordered or videos added at the top on YouTube] → numbers keep
  the order at first sight and later additions append, so Plex drifts from
  the live YouTube order (the web UI follows the live position). Accepted:
  renumbering would break Plex watch state.
- [Two passes of one playlist store the same new videos concurrently] → both
  could read the same `last_episode`; the same race already exists for the
  added rows themselves, so it is accepted, as the DDD rule on find-then-save
  races says. An HTTP playlist edit overlapping a pass is not a risk:
  `update` doesn't write `last_episode`, and `record_last_episode` never
  lowers it.
- [More than 99 videos of one channel on one day] → numbering the 100th
  fails with a validation error (visible, retried) instead of spilling into
  the next day's prefix; the video is marked errored like a failed
  download, never left in progress.
- [A test library already laid out by this branch] → no back-compat: wipe the
  database and `videos/`, then sync again.

## Migration Plan

1. Deploy the image; `serve` applies `0008_episode_numbers.sql` on start as
   usual, and new downloads already land in the TV layout.
2. Stop the daemon, run `docker compose run --rm yarrtube yarrtube
   migrate-layout --dry-run`, review, run without `--dry-run`, start the
   daemon.
3. In Plex: create a TV Shows library (Plex NFO Series agent, Seasons: hide
   for single-season series, local assets on) on the same folders, delete the
   old Movies library.
4. Rollback: previous image plus restoring the files from backup; the
   migration is not reversible by the tool.

## Open Questions

None that change the specs or the task list.

## Files

- `migrations/0008_episode_numbers.sql` — `season`/`episode` columns on `videos`, `last_episode` on `playlists`, playlist videos numbered by position.
- `src/infrastructure/shared/sqlite_migrations.rs` — register 0008.
- `src/domain/video/episode_number.rs` — `EpisodeNumber` enum.
- `src/domain/video/channel_episode_number.rs` — date-based value object (was `EpisodeNumber`).
- `src/domain/video/playlist_episode_number.rs` — playlist-order value object.
- `src/domain/playlist/playlist.rs` — `last_episode` field, `next_episode_number`, `with_last_episode`.
- `src/infrastructure/repositories/sqlite_playlist_repository.rs` — persist `last_episode` on insert; `update` doesn't write it; `record_last_episode` raises it.
- `src/domain/services/playlist_video_reconciler.rs` — number new videos in `add_video`.
- `src/domain/video/video.rs` — `episode_number` field + transition.
- `src/domain/video/video_filename.rs` — collision helpers and episode-name helpers removed (see `episode_paths.rs`).
- `src/domain/video/episode_paths.rs` — `EpisodePaths`, the one source of episode file names.
- `src/domain/video/thumbnail_filename.rs` — deleted.
- `src/domain/video/video_output_entry.rs` — layout helpers (season folder, nfo path, legacy detection).
- `src/domain/video/events.rs` — unchanged; `VideoDownloaded.folder` no longer read.
- `src/domain/video/mod.rs` — exports.
- `src/domain/video_metadata/nfo.rs` — `render_episode_nfo`, `render_show_nfo`.
- `src/domain/video_metadata/mapping.rs` — `resolve_sorttitle` kept (DB column), no NFO use.
- `src/domain/show/mod.rs`, `src/domain/show/show.rs` — `Show` (title, uniqueid, poster source) built from a channel or playlist.
- `src/domain/services/show_file_writer.rs` — writes `tvshow.nfo` + `poster.jpg`.
- `src/domain/services/video_relocator.rs` — assign number + move file set + rewrite NFO + record.
- `src/domain/services/layout_migrator.rs` — iterates downloaded videos, dry run, report.
- `src/domain/services/video_downloader.rs` — staging flow.
- `src/domain/services/thumbnail_fetcher.rs` — staging flow + number assignment.
- `src/domain/services/internal_video_reconciler.rs` — recursive sweep, new protected set, legacy relocation on recovery.
- `src/domain/services/video_file_deleter.rs` — delete by base name; legacy by top-level entry.
- `src/domain/services/channel_video_reconciler.rs`, `playlist_video_reconciler.rs` — call `ShowFileWriter`.
- `src/domain/services/plex_folder_scanner.rs` — no section ids; discovers sections.
- `src/domain/services/plex_collection_reconciler.rs`, `plex_collection_deleter.rs` — deleted.
- `src/domain/plex/{plex_collection,plex_item,plex_match_candidate}.rs` — deleted; `plex_section.rs`, `plex_folder_path.rs` stay.
- `src/domain/services/mod.rs`, `src/domain/plex/mod.rs`, `src/domain/mod.rs` — module lists.
- `src/application/tasks/reconcile_plex_collections_task.rs` — deleted; `tasks/mod.rs` registry arg removed.
- `src/domain/task/task.rs` — `ReconcilePlexCollections` variant removed (and its payload/decoder); `web/src/lib/tasks.ts` label removed.
- `src/application/subscribers/delete_plex_collection_on_{playlist,channel}_deleted.rs` — deleted; `subscribers/mod.rs` registry arg removed.
- `src/application/subscribers/scan_plex_folder_on_video_downloaded.rs` — scans `output_dir`.
- `src/application/cli/mod.rs`, `src/application/cli/migrate_layout.rs`, `src/main.rs` — subcommand; a dry run migrates the schema, then plans.
- `src/serve.rs` — Plex wiring reduced to URL/token/videos path; `.downloads` created at startup; show file writer, relocator wired.
- `src/infrastructure/infrastructure_container.rs` — `show_file_repository`.
- `src/infrastructure/shared/ytdlp.rs` — staging output template, timestamp printing, `prepare_video_dir`/collision code removed.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs` — new port signatures + fake.
- `src/infrastructure/repositories/filesystem_video_file_repository.rs` — recursive list, move, delete set + fake.
- `src/infrastructure/repositories/filesystem_show_file_repository.rs` — new port + adapter (no write when unchanged) + fake (with a failing variant).
- `src/infrastructure/repositories/sqlite_video_repository.rs` — columns, `assign_episode_number`, `list_downloaded`.
- `src/infrastructure/repositories/sqlite_video_metadata_repository.rs` — NFO path instead of dir; episode rendering.
- `src/infrastructure/repositories/plex_collection_repository.rs` — renamed `plex_library_repository.rs`, trait trimmed to `list_sections` + `scan_path`, fake trimmed.
- `src/infrastructure/repositories/filesystem_channel_avatar_repository.rs` — `path_of(filename)`.
- `web/src/components/VideoDetail.tsx` (+ test), `web/src/lib/storageLocation.ts` (+ test) — storage location line.
- `web/src/App.tsx` — mobile sidebar top re-measured while open (the announcement bar may appear after it opened); `smoke-tests/tests/mobile-sidebar.spec.js` — late-announcement case.
- `src/domain/video_metadata/mod.rs` — re-exports `render_episode_nfo`, `render_show_nfo`; `src/domain/services/metadata_generator.rs`, `src/domain/video_metadata/video_metadata.rs`, `src/infrastructure/repositories/youtube_metadata_repository.rs` — doc comments only.
- Test-only: `src/application/http/{channels,playlists,tasks,videos}/mod.rs`, `src/application/subscribers/reconcile_on_{channel,playlist}_created.rs`, `src/application/tasks/{download_video,fetch_thumbnail,delete_video_file,reconcile_channel,reconcile_playlist}_task.rs` (behaviour tests and wiring); `src/application/tasks/log_capture.rs` (rebuilds the tracing interest cache).
- `web/src/lib/tasks.ts` (+ test), `web/src/components/TasksView.test.tsx` — drop the collections task label.
- `doc/PLEX.md`, `doc/INSTALLATION.md`, `doc/ARCHITECTURE.md`, `README.md`, `CLAUDE.md`, `.env.example`, `scripts/run-local.sh` — docs and config.

## Types & Signatures

```rust
// src/domain/video/channel_episode_number.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChannelEpisodeNumber { season: i32, episode: u32 }   // episode = MMDD * 100 + index
impl ChannelEpisodeNumber {
    pub fn new(season: i32, episode: u32) -> Result<Self, ValidationError>; // 4-digit year, valid MMDD, index 01..=99
    pub fn first_of_day(published_at: DateTime<Utc>) -> Self;          // index 01
    pub fn season(&self) -> i32;
    pub fn episode(&self) -> u32;
    pub fn same_day_prefix(&self) -> u32;                               // MMDD
}

// src/domain/video/playlist_episode_number.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaylistEpisodeNumber { episode: u32 }               // season is always 1
impl PlaylistEpisodeNumber {
    pub const SEASON: i32 = 1;
    pub fn new(episode: u32) -> Result<Self, ValidationError>;   // episode >= 1
    pub fn episode(&self) -> u32;
    pub fn next(&self) -> Result<Self, ValidationError>;
}

// src/domain/video/episode_number.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EpisodeNumber { Channel(ChannelEpisodeNumber), Playlist(PlaylistEpisodeNumber) }
impl EpisodeNumber {
    pub fn from_columns(season: i32, episode: u32) -> Result<Self, ValidationError>; // season 1 -> Playlist
    pub fn season(&self) -> i32;
    pub fn episode(&self) -> u32;
    pub fn code(&self) -> String;            // "S2026E031501" / "S01E007" / "S01E1000"
    pub fn season_folder(&self) -> String;   // "Season 2026" / "Season 1"
}
impl From<ChannelEpisodeNumber> for EpisodeNumber;
impl From<PlaylistEpisodeNumber> for EpisodeNumber;

// src/domain/playlist/playlist.rs
pub struct Playlist { /* existing */ pub last_episode: Option<PlaylistEpisodeNumber> }
impl Playlist {
    pub fn next_episode_number(&self) -> Result<PlaylistEpisodeNumber, ValidationError>; // 1 when None
    pub fn with_last_episode(self, number: PlaylistEpisodeNumber) -> Self;
}

// src/domain/video/video.rs
pub struct Video { /* existing */ pub episode_number: Option<EpisodeNumber> }
impl Video {
    pub fn with_episode_number(self, number: EpisodeNumber, now: DateTime<Utc>) -> Self;
    pub fn relocate_to_episode(self, number: EpisodeNumber, filename: impl Into<String>, thumbnail_filename: Option<String>, now: DateTime<Utc>) -> Self;
    pub fn is_in_legacy_layout(&self) -> bool;   // recorded filename not under "Season "
}

// src/domain/video/episode_paths.rs
pub struct EpisodePaths { season_folder: String, base_name: String }
impl EpisodePaths {
    pub fn new(number: &EpisodeNumber, title: &str) -> Self;
    pub fn stem(&self) -> String;                  // "Season 2026/S2026E031501 - <sanitized>"
    pub fn media(&self, extension: &str) -> String;
    pub fn thumbnail(&self) -> String;             // stem + ".jpg"
    pub fn nfo(&self) -> String;                   // stem + ".nfo"
    pub fn thumb(&self) -> String;                 // "<base>.jpg", as the NFO references it
}
// src/domain/video/video_filename.rs — removed: video_folder_candidates, collision_suffixed_folder

// src/domain/video/video_output_entry.rs
pub fn top_level_entry(relative_path: &str) -> &str;            // kept for legacy
pub fn resolve_output_dir(videos_path: &str, path: &str) -> PathBuf;
pub fn is_episode_path(relative_path: &str) -> bool;            // "Season <digits>/S<digits>E<digits> - …"
pub fn is_season_folder(entry: &str) -> bool;                   // "Season " + digits only
pub fn nfo_path_for(relative_path: &str) -> String;             // ".nfo" sibling for episode paths
pub fn staging_dir(videos_path: &str, video_id: &VideoRecordId) -> PathBuf; // "<root>/.downloads/<record id>"
pub const SHOW_NFO_FILENAME: &str = "tvshow.nfo";
pub const SHOW_POSTER_FILENAME: &str = "poster.jpg";

// src/domain/show/show.rs
pub struct Show { pub title: String, pub uniqueid: String, pub poster_source: Option<PathBuf> }
impl Show {
    pub fn from_channel(channel: &Channel, poster_source: Option<PathBuf>) -> Self;   // poster_source = ChannelAvatarRepository::path_of(avatar)
    pub fn from_playlist(playlist: &Playlist) -> Self;
}

// src/domain/video_metadata/nfo.rs
pub fn render_episode_nfo(metadata: &VideoMetadata, number: &EpisodeNumber) -> String;
pub fn render_show_nfo(show: &Show) -> String;
// removed: render_movie_nfo

// src/domain/services/show_file_writer.rs
pub struct ShowFileWriter { show_file_repository: Arc<dyn ShowFileRepository>, channel_avatar_repository: Arc<dyn ChannelAvatarRepository>, videos_path: String }
pub trait ShowFileWriterApi {
    fn write_channel_show(&self, channel: &Channel) -> anyhow::Result<()>;
    fn write_playlist_show(&self, playlist: &Playlist) -> anyhow::Result<()>;
}

// src/domain/services/video_relocator.rs
pub struct VideoRelocator { video_repository, video_file_repository, video_metadata_repository, clock }
pub enum Relocation { Moved { from: String, to: String, number: EpisodeNumber }, AlreadyInPlace, Skipped(SkipReason) }
pub enum SkipReason { NoPublishTimestamp, FileMissing }
pub trait VideoRelocatorApi {
    /// Uses the recorded number, else assigns a channel number from `published_at`
    /// (skipping with NoPublishTimestamp when neither exists), moves media/thumbnail/nfo
    /// into the season folder, rewrites the NFO, records the new filenames.
    fn relocate(&self, video: &Video, published_at: Option<DateTime<Utc>>, output_dir: &Path) -> anyhow::Result<Relocation>;
    /// Dry run: the same outcome `relocate` would have, numbered from
    /// `next_episode_number` and the numbers already `planned` this run; changes nothing.
    fn plan(&self, video: &Video, published_at: Option<DateTime<Utc>>, output_dir: &Path, planned: &[EpisodeNumber]) -> anyhow::Result<Relocation>;
}

// src/domain/services/layout_migrator.rs
pub struct LayoutMigrator { video_repository, playlist_repository, channel_repository, playlist_video_repository, channel_video_repository, video_metadata_repository, relocator: Arc<VideoRelocator>, show_file_writer: Arc<ShowFileWriter>, videos_path: String }
pub struct MigrationReport { pub moved: Vec<(String, String)>, pub skipped: Vec<(String, SkipReason)>, pub failed: Vec<(String, String)> }
pub trait LayoutMigratorApi { fn migrate(&self, dry_run: bool) -> anyhow::Result<MigrationReport>; }
// order: per source, per publish day, by published_at ascending

// src/domain/services/video_downloader.rs (private steps renamed)
fn run_download(&self, video: &Video, quality: Quality, output_dir: &Path) -> anyhow::Result<DownloadAttempt>;
fn place_download(&self, video: Video, downloaded: DownloadedVideo, quality: Quality, output_dir: &Path, metadata: Option<VideoMetadata>) -> anyhow::Result<()>;
// = assign number -> write nfo at episode path -> move media (+thumb) -> record -> publish VideoDownloaded
fn discard_staging(&self, video: &Video);

// src/domain/services/thumbnail_fetcher.rs
fn fetch_thumbnail(&self, video: &Video, output_dir: &Path) -> anyhow::Result<ThumbnailFetch>;
fn place_thumbnail(&self, video: &Video, fetched: FetchedThumbnail, output_dir: &Path) -> anyhow::Result<()>;
// = assign number -> move jpg to episode path -> update_thumbnail

// src/domain/services/plex_folder_scanner.rs
pub struct PlexFolderScanner { videos_root: PathBuf, plex_videos_root: String, plex_library_repository: Arc<dyn PlexLibraryRepository> }
pub fn new(videos_root: impl Into<PathBuf>, plex_videos_root: impl Into<String>, repo: Arc<dyn PlexLibraryRepository>) -> Self;
pub trait PlexFolderScannerApi { fn scan_folder(&self, folder: &Path) -> anyhow::Result<()>; }

// src/infrastructure/shared/ytdlp.rs
pub struct DownloadedVideo { pub media_path: PathBuf, pub thumbnail_path: Option<PathBuf>, pub duration_seconds: Option<i64>, pub published_at: Option<DateTime<Utc>>, pub sabr_notice: Option<String> }
pub struct FetchedThumbnail { pub path: PathBuf, pub published_at: Option<DateTime<Utc>> }
pub fn download_video(ytdlp_path: &Path, video_url: &str, quality: Quality, staging_dir: &Path) -> anyhow::Result<DownloadAttempt>;
pub fn fetch_thumbnail(ytdlp_path: &Path, video_url: &str, staging_dir: &Path) -> anyhow::Result<ThumbnailFetch>;
// -o "%(id)s.%(ext)s", --print "%(timestamp)s", --print "%(upload_date)s"; removed: prepare_folder/prepare_video_dir/create_fresh_video_dir
// download_video: thumbnail_path = the `<media stem>.jpg` --write-thumbnail left in staging_dir, None when absent

// src/infrastructure/repositories/youtube_video_downloader_repository.rs
pub trait VideoDownloaderRepository {
    fn download(&self, video_url: &str, quality: Quality, staging_dir: &Path) -> anyhow::Result<DownloadAttempt>;
    fn fetch_thumbnail(&self, video_url: &str, staging_dir: &Path) -> anyhow::Result<ThumbnailFetch>;
    fn diagnose(&self, video_url: &str) -> anyhow::Result<Option<String>>;
}

// src/infrastructure/repositories/filesystem_video_file_repository.rs
pub trait VideoFileRepository {
    fn delete(&self, output_dir: &Path, relative_path: &str) -> anyhow::Result<bool>;   // file or legacy dir
    fn list(&self, output_dir: &Path) -> anyhow::Result<Vec<String>>;                   // top-level entries + "Season */<file>" entries
    fn delete_dir_recursive(&self, dir: &Path) -> anyhow::Result<()>;
    fn file_exists(&self, output_dir: &Path, relative_path: &str) -> bool;
    fn move_file(&self, from: &Path, to: &Path) -> anyhow::Result<()>;                   // creates parent, rename
    fn remove_if_empty(&self, dir: &Path) -> anyhow::Result<()>;
}

// src/infrastructure/repositories/filesystem_show_file_repository.rs
pub trait ShowFileRepository {   // both leave an identical existing file untouched
    fn write_show_nfo(&self, show_dir: &Path, nfo: &str) -> anyhow::Result<()>;
    fn copy_poster(&self, show_dir: &Path, source: &Path) -> anyhow::Result<()>;
}
pub struct FilesystemShowFileRepository;

// src/infrastructure/repositories/sqlite_video_repository.rs
pub trait VideoRepository { /* existing */
    fn assign_episode_number(&self, id: &VideoRecordId, published_at: DateTime<Utc>) -> anyhow::Result<EpisodeNumber>; // fails for an unnumbered video in no channel
    fn list_downloaded(&self) -> anyhow::Result<Vec<Video>>;
    fn next_episode_number(&self, id: &VideoRecordId, published_at: DateTime<Utc>) -> anyhow::Result<EpisodeNumber>; // read-only peek, same rule as assign
}

// src/infrastructure/repositories/sqlite_playlist_repository.rs
pub trait PlaylistRepository { /* existing; update no longer writes last_episode */
    fn record_last_episode(&self, id: &PlaylistId, number: PlaylistEpisodeNumber) -> anyhow::Result<()>; // MAX, never lowers
}

// src/infrastructure/repositories/sqlite_video_metadata_repository.rs
pub trait VideoMetadataRepository {
    fn save(&self, video_id: &VideoRecordId, metadata: &VideoMetadata, number: &EpisodeNumber, nfo_path: &Path) -> anyhow::Result<()>;
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>>;
    fn write_nfo(&self, metadata: &VideoMetadata, number: &EpisodeNumber, nfo_path: &Path) -> anyhow::Result<()>;
    fn remove_nfo(&self, nfo_path: &Path) -> anyhow::Result<()>;
}

// src/infrastructure/repositories/plex_library_repository.rs (was plex_collection_repository.rs)
pub trait PlexLibraryRepository {
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>>;
    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()>;
}
pub struct HttpPlexLibraryRepository { /* as today */ }

// src/infrastructure/repositories/filesystem_channel_avatar_repository.rs
pub trait ChannelAvatarRepository { /* existing */ fn path_of(&self, filename: &str) -> PathBuf; }

// src/application/cli/mod.rs
pub enum Commands { Serve, UpdateYtdlp, MigrateLayout { #[arg(long)] dry_run: bool } }
// src/application/cli/migrate_layout.rs
pub fn run(dry_run: bool) -> ExitCode;   // migrate schema -> build infrastructure -> LayoutMigrator::migrate; one line per move/skip/failure + summary; non-zero on any failure

// src/serve.rs
struct PlexIntegration { repository: Arc<dyn PlexLibraryRepository>, plex_videos_path: String }
fn plex_integration() -> Option<PlexIntegration>;   // URL + token + videos path, warn on partial
fn show_file_writer(infra) -> Arc<ShowFileWriter>;
fn video_relocator(infra) -> Arc<VideoRelocator>;
pub(crate) fn run_startup_migrations() -> anyhow::Result<()>;
pub(crate) fn build_infrastructure() -> anyhow::Result<InfrastructureContainer>;
```

```sql
-- migrations/0008_episode_numbers.sql
ALTER TABLE videos ADD COLUMN season INTEGER;
ALTER TABLE videos ADD COLUMN episode INTEGER;
ALTER TABLE playlists ADD COLUMN last_episode INTEGER;
UPDATE videos SET season = 1, episode = (
    SELECT n FROM (SELECT video_id, ROW_NUMBER() OVER (
        PARTITION BY playlist_id ORDER BY position, id) AS n FROM playlist_videos) AS numbered
    WHERE numbered.video_id = videos.id)
WHERE id IN (SELECT video_id FROM playlist_videos);
UPDATE playlists SET last_episode = (
    SELECT NULLIF(COUNT(*), 0) FROM playlist_videos WHERE playlist_id = playlists.id);
DELETE FROM tasks WHERE task_type = 'reconcile_plex_collections';
```

```ts
// web/src/components/VideoDetail.tsx
const location = storageLocation(basePath, video.filename)  // "channels/x/Season 2026" or basePath
// web/src/lib/storageLocation.ts
export function storageLocation(basePath: string, filename: string | null): string
```

## Call Stack

**Download**
`DownloadVideoTask::handle(payload)` → `VideoDownloader::download(video_id, quality, output_dir, is_last_attempt)`
→ `fetch_metadata(video)` (Data API, best effort)
→ `VideoDownloaderRepository::download(url, quality, staging_dir(videos_path, video.id))` → `ytdlp::download_video`
→ on `Succeeded(downloaded)`: `place_download(video, downloaded, quality, output_dir, metadata)`:
  `VideoRepository::assign_episode_number(id, downloaded.published_at or now)` (returns existing if set)
  → paths from `EpisodePaths::new(number, title)`
  → `VideoMetadataRepository::write_nfo(metadata, number, output_dir/paths.nfo())` when metadata
  → `VideoFileRepository::move_file(downloaded.media_path, output_dir/paths.media(ext))`, same for thumbnail (`paths.thumbnail()`)
  → `VideoRepository::update(video.mark_downloaded(...))` → `save_metadata` (`VideoMetadataRepository::save`, re-fetching the metadata when the first fetch failed)
  → `discard_staging(video)` → `EventPublisher::publish(VideoDownloaded { video_id, output_dir, folder: season folder })` (`folder` unread)
→ placing fails (assign, move or record): `record_placement_errored` → `discard_staging(video)`, re-read the video (keeps a number already assigned), `record_errored(.., is_last_attempt)`; a video deleted meanwhile gets nothing recorded.
→ on `Failed`: `discard_staging(video)` (`delete_dir_recursive(staging_dir)`), then today's failure handling.
→ video or source deleted meanwhile: `discard_staging`, no record.

**Thumbnail ahead**
`FetchThumbnailTask::handle` → `ThumbnailFetcher::fetch(video, output_dir)` → `VideoDownloaderRepository::fetch_thumbnail(url, staging_dir)`
→ `place_thumbnail`: `assign_episode_number(id, fetched.published_at)` → `move_file(fetched.path, output_dir/EpisodePaths::new(..).thumbnail())` → `VideoRepository::update_thumbnail(id, relative_path, now)`; then, whatever the outcome, `delete_dir_recursive(staging_dir)`.

**Reconcile (channel or playlist)**
`ReconcileChannelTask::handle` / `ReconcileOnChannelCreated::handle` → `ChannelVideoReconciler::reconcile*` → `find_channel` → `ShowFileWriter::write_channel_show(channel)`, a failure logged and the pass continuing (`render_show_nfo(Show::from_channel(..))` → `ShowFileRepository::write_show_nfo`, `copy_poster(show_dir, ChannelAvatarRepository::path_of(avatar))`) → existing membership sync → `InternalVideoReconciler::reconcile(desired, delta)`:
  `read_actual_state` uses `VideoFileRepository::list` (top-level + season entries) and `protected_entries(videos)` = `tvshow.nfo`, `poster.jpg`, `Season *` dirs, recorded filename/thumbnail, `nfo_path_for(filename)`, every `<EpisodePaths::stem()>.*` of videos with a number, legacy top-level entries
  → `generate_missing_metadata` → for a video with `is_in_legacy_layout()` and metadata now present: `VideoRelocator::relocate(video, Some(metadata.published_at), output_dir)`
  → `delete_orphaned_files` deletes the rest.
Playlist path identical via `PlaylistVideoReconciler` and `write_playlist_show`, except new videos: for each new item in YouTube position order, `add_video(playlist, ..)` → `number = playlist.next_episode_number()` → `VideoRepository::save(Video::create(..).with_episode_number(number.into(), now))` → `PlaylistVideoRepository::save` → `PlaylistRepository::record_last_episode(playlist.id, number)`, `playlist.with_last_episode(number)` threaded into the next `add_video`.

**Delete video files**
`DeleteVideoFileTask::handle(filename, thumbnail_filename, output_dir)` → `VideoFileDeleter::delete_video_file` → if `is_episode_path(filename)`: `delete(output_dir, filename)`, `delete(output_dir, nfo_path_for(filename))`, `delete(output_dir, thumbnail)`; else `delete(output_dir, top_level_entry(filename))` as today.

**Plex scan**
`ScanPlexFolderOnVideoDownloaded::handle(payload)` → `PlexFolderScanner::scan_folder(Path::new(payload.output_dir))` → `plex_folder_path(videos_root, plex_videos_root, folder)` → `PlexLibraryRepository::list_sections()` → for each section containing it `scan_path(section.id, plex_path)`.

**Migrate layout**
`main` → `cli::migrate_layout::run(dry_run)` → `run_startup_migrations()` (dry run too) → `build_infrastructure()` → `LayoutMigrator::migrate(dry_run)`:
for each source (`PlaylistRepository::list`, `ChannelRepository::list`): `ShowFileWriter::write_*_show` (skipped on dry run; an error goes to `failed` under the source path and the source's videos are still processed) → its downloaded videos (`list_downloaded` joined via link repositories) in legacy layout, with `VideoMetadataRepository::find` → sorted by `published_at` (unknown last) → real run: `VideoRelocator::relocate(video, published_at, output_dir)`; dry run: `VideoRelocator::plan(video, published_at, output_dir, planned)` with the numbers of this source's earlier `Moved` plans; no recorded number and `None` metadata (a channel video) → `Skipped(NoPublishTimestamp)`; missing file → `Skipped(FileMissing)`; error → `failed`.
`relocate`: legacy file missing → `Skipped(FileMissing)` unless the media already sits at the recorded number's target (`is_half_moved`), which is completed; recorded number, else `assign_episode_number` → `move_file` media → `move_file` thumbnail (each skipped when already at its target) → `VideoMetadataRepository::save` at the new NFO path (thumb renamed) when metadata is recorded, `remove_nfo(old movie.nfo)` → `remove_if_empty(old folder)` → `VideoRepository::update(video.relocate_to_episode(..))`.
`plan`: recorded number, else `next_episode_number` → bumped past `planned` numbers of the same season and day → `Moved` with the would-be paths; no write.

## Test Plan

Behaviour tests (application layer):

1. `it_should_save_a_downloaded_video_as_an_episode_in_its_season_folder` — download task: media, jpg and nfo at `Season 2026/S2026E031501 - Title.*`, recorded filenames relative, number recorded, staging dir gone, `VideoDownloaded` published with the show dir.
2. `it_should_write_the_episode_nfo_before_moving_the_media_into_place` — fake file repository records the order: nfo write precedes media move.
3. `it_should_number_two_same_day_videos_of_one_show_consecutively` — two downloads of videos published the same day get `031501` and `031502`.
4. `it_should_reuse_the_number_recorded_by_the_thumbnail_fetch` — video already numbered by thumbnail fetch; download lands next to the jpg with the same base name.
5. `it_should_number_from_the_upload_date_when_no_timestamp_is_reported` — `published_at` from `upload_date` only.
6. `it_should_download_under_its_episode_name_without_metadata` — Data API fails before and after: file placed, no nfo, no metadata row.
7. `it_should_discard_the_staging_dir_when_the_download_fails` — staging removed, no file in the season folder, nfo not present.
8. `it_should_discard_the_download_when_the_video_was_deleted_meanwhile` — staging removed, nothing recorded, nothing in the show dir.
9. `it_should_place_a_prefetched_thumbnail_as_the_episode_image` — thumbnail task: jpg at `Season 2026/<base>.jpg`, number and thumbnail recorded, status untouched.
10. `it_should_leave_the_video_untouched_when_the_thumbnail_fetch_fails` — staging removed, no number assigned, no thumbnail recorded.
11. `it_should_write_the_show_files_when_a_channel_is_created` — channel created subscriber: `tvshow.nfo` with name and channel id, `poster.jpg` bytes equal the avatar.
12. `it_should_write_the_show_nfo_without_a_poster_for_a_playlist` — playlist created subscriber.
13. `it_should_rewrite_missing_show_files_on_a_recurring_reconcile` — reconcile channel task after `tvshow.nfo` removed.
14. `it_should_not_sweep_show_files_season_folders_or_episode_files_of_recorded_videos` — reconcile playlist task: listing with `tvshow.nfo`, `poster.jpg`, `Season 2026/<base>.{mp4,jpg,nfo}` for a downloaded video and an orphan `Season 2026/stray.mp4`; only the stray is deleted.
15. `it_should_protect_files_sharing_the_base_name_of_an_in_progress_numbered_video` — nfo of an in-progress numbered video is kept.
16. `it_should_relocate_a_legacy_video_when_its_metadata_is_recovered` — reconcile channel task: downloaded video at `Title/Title.mp4` with no metadata; metadata now fetchable; files moved under the season folder, `movie.nfo` removed, empty folder removed, filenames recorded, number from `published_at`.
17. `it_should_leave_a_legacy_video_in_place_when_its_metadata_is_still_unavailable` — no move, no reset.
18. `it_should_reset_a_numbered_video_whose_file_is_missing_without_dropping_its_number` — reconcile: PENDING, filenames cleared, number kept.
19. `it_should_delete_an_episodes_media_thumbnail_and_nfo` — delete video file task with episode paths; show files untouched.
20. `it_should_delete_a_legacy_per_video_folder` — delete task with legacy path removes the folder.
21. `it_should_scan_the_show_folder_in_every_section_containing_it` — scan subscriber: two sections, one containing.
22. `it_should_skip_the_scan_when_no_section_contains_the_show_folder` — warning, `Ok`.
23. `it_should_register_the_scan_subscriber_only_with_url_token_and_videos_path` — serve wiring (replaces the section-id tests).
24. `it_should_migrate_a_legacy_library_into_the_tv_layout` — migrate-layout: two sources, three videos incl. a `[id]`-suffixed folder; files moved, rows updated, show files written, old folders gone, report lists three moves.
25. `it_should_number_a_same_day_batch_in_publish_time_order` — migrate-layout: three videos published seconds apart get `01..03` by time.
26. `it_should_skip_videos_without_metadata_or_without_a_file` — report lists both skips with reasons, nothing moved for them.
27. `it_should_change_nothing_on_a_dry_run` — same fixtures, including a same-day batch planned `01..03`: report equal to the real plan, filesystem and rows untouched.
28. `it_should_move_nothing_on_a_second_run` — rerun reports zero moves, rewrites show files.
29. `it_should_report_a_failed_move_and_continue` — failing file repository for one video: that row unchanged, others moved, report has one failure.
30. `it_should_fail_if_a_video_failed_to_move` — CLI run returns failure code.
31. `it_should_not_register_the_collections_task_or_subscribers` — serve: `reconcile_plex_collections` absent from handlers, no deletion subscribers.

Playlist numbering (decisions 9–11):

32. `it_should_number_new_playlist_videos_in_playlist_order` — reconcile on playlist created: three items at positions 0..2 with publish times in another order get season 1 episodes 1..3 by position; playlist `last_episode` = 3.
33. `it_should_number_a_video_added_to_the_playlist_after_the_last_one` — reconcile playlist task: `last_episode` 5, new item at position 0 → episode 6.
34. `it_should_not_reuse_the_number_of_a_removed_video` — the video holding 6 leaves; a later new item → 7.
35. `it_should_keep_episode_numbers_when_the_playlist_is_reordered` — positions swap; numbers and filenames unchanged.
36. `it_should_save_a_playlist_video_under_its_playlist_episode_number` — download task: numbered season 1 / 7, reported publish 2014 → `Season 1/S01E007 - Title.*`, number unchanged.
37. `it_should_move_legacy_playlist_videos_under_their_recorded_numbers` — migrate-layout: playlist videos numbered by position, publish order reversed → moved to `S01E001`/`S01E002` by position.
38. `it_should_move_a_playlist_video_without_metadata` — migrate-layout: numbered playlist video, no metadata → moved, no NFO, not skipped.

Audit fixes:

39. `it_should_complete_a_video_whose_media_an_interrupted_run_already_moved` — migrate-layout: media already at `S01E001`, jpg still legacy → only the jpg moves, row relocated, old folder removed.
40. `it_should_report_show_files_it_cannot_write_and_still_move_the_videos` — migrate-layout: failing show file repository → failure keyed by the source dir, video moved.
41. `it_should_sync_the_channel_even_if_its_show_files_cannot_be_written` — reconcile channel task: failing show file repository → new video stored, next reconcile scheduled.
42. `it_should_plan_the_same_day_of_different_years_independently_on_a_dry_run` — 2025-03-15 and 2026-03-15 both plan index `01`.
43. `it_should_plan_past_a_later_number_already_recorded_that_day_on_a_dry_run` — planned `031501`, recorded `031502` → plans `031503`.
44. `it_should_mark_errored_if_the_download_cannot_be_moved_into_place_on_the_last_attempt` — download task: media move fails → video `Errored` with its assigned number kept, staging dir discarded, no `VideoDownloaded`.
45. `it_should_recognize_an_episode_file_in_its_season_folder` — `Season 2026/S2026E031501 - X.mp4`, `Season 1/S01E007 - X.jpg` are episode paths.
46. `it_should_treat_a_legacy_folder_titled_like_a_season_as_legacy` — `Season 2 Finale/Season 2 Finale.mp4`, `Season 2/Season 2.mp4` are legacy.
47. `it_should_only_take_a_season_and_number_as_a_season_folder` — `Season 2026`, `Season 1` yes; `Season 2 Finale`, `Season Finale` no.

Infrastructure tests:

- `ytdlp`: `it_should_download_into_the_staging_dir_with_the_video_id_as_name`, `it_should_report_the_publish_timestamp_printed_by_ytdlp`, `it_should_fall_back_to_the_upload_date_when_the_timestamp_is_na`, `it_should_report_the_staged_thumbnail_written_next_to_the_download`, `it_should_report_the_written_thumbnail_path` (thumbnail fetch).
- `FilesystemVideoFileRepository`: `it_should_list_top_level_entries_and_season_files`, `it_should_move_a_file_creating_its_parent`, `it_should_remove_an_empty_dir_only`, `it_should_delete_a_file_or_a_dir_by_relative_path`.
- `FilesystemShowFileRepository`: `it_should_write_the_show_nfo_and_copy_the_poster`, `it_should_overwrite_an_existing_poster`, `it_should_leave_unchanged_show_files_untouched`, `it_should_overwrite_a_changed_show_nfo`.
- `SqliteVideoRepository`: `it_should_assign_the_first_index_of_the_day`, `it_should_assign_the_next_free_index_within_the_same_channel_and_day` (siblings via `channel_videos` only), `it_should_fail_to_number_an_unnumbered_playlist_video_by_publish_date`, `it_should_persist_and_read_a_playlist_episode_number`, `it_should_number_copies_in_different_shows_independently`, `it_should_return_the_existing_number_when_already_assigned`, `it_should_persist_and_read_the_episode_number`, `it_should_list_downloaded_videos`, `it_should_peek_the_next_episode_number_without_recording_it`.
- `SqliteVideoMetadataRepository`: `it_should_write_an_episode_nfo_with_season_episode_and_aired`, `it_should_write_the_nfo_at_the_given_path`, `it_should_remove_the_nfo_at_the_given_path`.
- `sqlite_migrations`: `it_should_drop_queued_plex_collection_reconciles_when_migrating`, `it_should_add_the_episode_number_columns`, `it_should_number_existing_playlist_videos_by_position` (ties by insertion, channel videos untouched, `last_episode` = count, `NULL` for an empty playlist).
- `SqlitePlaylistRepository`: `it_should_raise_the_last_episode`, `it_should_never_lower_the_last_episode`, `it_should_keep_the_last_episode_when_updating_from_a_stale_read`.
- Value objects: `PlaylistEpisodeNumber` (`it_should_reject_episode_zero`, `it_should_give_the_next_number`), `EpisodeNumber` (`it_should_render_a_playlist_code_padded_to_three_digits`, `it_should_widen_a_playlist_code_past_999`, `it_should_decode_season_one_as_a_playlist_number`), `ChannelEpisodeNumber` (`it_should_accept_a_day_and_index_within_range`, `it_should_fail_if_the_season_is_not_a_four_digit_year`, `it_should_fail_if_the_episode_is_not_a_day_and_index`, `it_should_fail_to_number_past_the_99th_video_of_a_day`, `it_should_start_the_day_at_index_one`), `EpisodePaths` (`it_should_name_every_file_of_a_channel_episode_after_its_stem`, `it_should_name_a_playlist_episode_in_season_one`).
- `HttpPlexLibraryRepository`: existing `list_sections` and `scan_path` tests kept; collection tests removed.
- `FilesystemChannelAvatarRepository`: `it_should_resolve_the_path_of_a_stored_avatar`.
- web `VideoDetail.test.tsx`: `shows the season folder as the storage location`, `shows the source path when the video has no file`; `storageLocation.test.ts` for the pure function.
- smoke `mobile-sidebar.spec.js`: `mobile sidebar stays closable when the announcement appears after it opened`.
