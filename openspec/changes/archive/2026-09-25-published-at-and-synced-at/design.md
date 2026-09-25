## Files

- `migrations/0003_published_at_and_synced_at.sql`: new columns, backfills, and the `premiered`/`year` drop.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers migration 0003; migration backfill tests.
- `src/domain/video/video.rs`: `Video.synced_at`, set by `mark_downloaded`, cleared by `reset_for_redownload`.
- `src/domain/video_metadata/video_metadata.rs`: `published_at`, `created_at`, `updated_at` replace `premiered`/`year`; `premiered()`/`year()` derive the NFO values.
- `src/domain/video_metadata/mapping.rs`: `build_video_metadata` takes `now` and passes `published_at` through.
- `src/domain/video_metadata/nfo.rs`: renders `premiered`/`year` from the derived accessors.
- `src/domain/services/video_downloader.rs`: passes `clock.now()` to `build_video_metadata`.
- `src/domain/services/playlist_video_reconciler.rs`: same, for metadata repair.
- `src/domain/services/channel_video_reconciler.rs`: same, for metadata repair.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: reads and writes `synced_at`.
- `src/infrastructure/repositories/sqlite_video_metadata_repository.rs`: reads and writes `published_at`/`created_at`/`updated_at`; the upsert keeps the stored `created_at`.
- `src/application/http/videos/dto.rs`: `VideoResponse.synced_at`.

## Types & Signatures

```rust
// domain/video/video.rs (signatures of transitions unchanged)
pub struct Video {
    // ...existing fields...
    pub synced_at: Option<DateTime<Utc>>, // Some(now) in mark_downloaded, None in create/reset_for_redownload
}

// domain/video_metadata/video_metadata.rs
pub struct VideoMetadata {
    pub title: String,
    pub plot: String,
    pub studio: String,
    pub director: String,
    pub published_at: DateTime<Utc>,
    pub genre: Option<String>,
    pub tags: Vec<String>,
    pub uniqueid: String,
    pub thumb: Option<String>,
    pub sorttitle: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl VideoMetadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: impl Into<String>,
        plot: impl Into<String>,
        studio: impl Into<String>,
        director: impl Into<String>,
        published_at: DateTime<Utc>,
        genre: Option<String>,
        tags: Vec<String>,
        uniqueid: impl Into<String>,
        thumb: Option<String>,
        sorttitle: impl Into<String>,
        now: DateTime<Utc>, // created_at = updated_at = now
    ) -> Self;
    pub fn premiered(&self) -> String; // published_at as "%Y-%m-%d"
    pub fn year(&self) -> i32;          // published_at.year()
}

// domain/video_metadata/mapping.rs
pub fn build_video_metadata(
    youtube_id: &VideoId,
    metadata: &YoutubeMetadata,
    sorttitle: impl Into<String>,
    thumbnail_filename: Option<String>,
    now: DateTime<Utc>,
) -> VideoMetadata;

// application/http/videos/dto.rs
pub struct VideoResponse {
    // ...existing fields...
    pub synced_at: Option<DateTime<Utc>>,
}
```

```sql
-- migrations/0003_published_at_and_synced_at.sql
ALTER TABLE videos ADD COLUMN synced_at TEXT;
UPDATE videos SET synced_at = updated_at WHERE status = 'DOWNLOADED';

ALTER TABLE video_metadata ADD COLUMN published_at TEXT NOT NULL DEFAULT '';
ALTER TABLE video_metadata ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
UPDATE video_metadata SET published_at = premiered || 'T00:00:00+00:00', updated_at = created_at;
ALTER TABLE video_metadata DROP COLUMN premiered;
ALTER TABLE video_metadata DROP COLUMN year;
```

The `DEFAULT ''` exists only because SQLite's `ADD COLUMN ... NOT NULL` requires a default; every row is backfilled by the following `UPDATE` and the app always writes both columns.

Replacing `premiered`/`year` with `published_at` leaves `movie.nfo` unchanged, so it is done in the walking skeleton and covered by the existing tests (e.g. `it_should_save_youtube_metadata`, the `nfo.rs` tests).

## Call Stack

Download (sets `synced_at`, writes metadata timestamps):
```
DownloadVideoTask::run(payload)
  -> VideoDownloader::download(video_id, quality, output_dir)
       -> video.start_download(now)
       -> started.mark_downloaded(quality, filename, thumbnail, duration, now)   // synced_at = Some(now)
       -> VideoRepository::update(&downloaded)                                  // writes synced_at
       -> VideoDownloader::generate_metadata(&downloaded, video_dir, thumbnail)
            -> YoutubeMetadataRepository::find(&youtube_id)                     // published_at
            -> build_video_metadata(&youtube_id, &metadata, sorttitle, thumbnail, clock.now())
            -> VideoMetadataRepository::save(&video.id, &video_metadata, video_dir)
                 -> render_movie_nfo(&metadata)                                 // premiered()/year()
                 -> INSERT ... ON CONFLICT (video_id) DO UPDATE SET ...,       // no created_at
                      published_at, updated_at
```

Reconcile redownload (clears `synced_at`):
```
ReconcilePlaylistTask::run(payload) / ReconcileChannelTask::run(payload)
  -> PlaylistVideoReconciler / ChannelVideoReconciler::reconcile(id)
       -> video.reset_for_redownload(now)            // synced_at = None
       -> VideoRepository::update(&reset)
```

Reconcile metadata repair:
```
PlaylistVideoReconciler::generate_metadata(video, output_dir, playlist_position)
ChannelVideoReconciler::generate_metadata(video, output_dir)
  -> build_video_metadata(&youtube_id, &metadata, sorttitle, thumbnail, clock.now())
  -> VideoMetadataRepository::save(&video.id, &video_metadata, video_dir)
```

Listing:
```
GET /playlists/{id}/videos | GET /channels/{handle}/videos
  -> VideoSearcher::list(id) | list_for_channel(handle)
  -> VideoResponse::from(video)                      // synced_at
```

## Test Plan

Behaviour tests:
1. `download_video_task::it_should_record_the_sync_time`: after a successful download, the stored `Video` equals the expected downloaded video with `synced_at: Some(fixed_timestamp())`.
2. `reconcile_playlist_task::it_should_clear_the_sync_time_of_videos_redownloaded`: a downloaded video whose file is missing is stored back as `Pending` with `synced_at: None`.
3. `download_video_task::it_should_record_when_metadata_was_generated`: the saved `VideoMetadata` has `created_at` and `updated_at` equal to the clock's now.
4. `download_video_task::it_should_keep_the_metadata_creation_time_when_regenerated`: with metadata already stored at an earlier `created_at`, a download re-saves it with the stored `created_at` kept and `updated_at` set to the clock's now.
5. `http::videos::it_should_include_the_sync_time_when_listing_videos`: listing a playlist with a downloaded video returns its `VideoResponse` with `synced_at` set.

Infrastructure tests (`sqlite_migrations`):
6. `it_should_backfill_the_sync_time_of_downloaded_videos_when_migrating`: from a database at migration 0002, a `DOWNLOADED` video gets `synced_at = updated_at` and a `PENDING` one gets `NULL`.
7. `it_should_backfill_the_publish_time_from_premiered_when_migrating`: an existing metadata row with `premiered = '2024-01-02'` gets `published_at = '2024-01-02T00:00:00+00:00'`.
8. `it_should_backfill_the_metadata_update_time_from_its_creation_time_when_migrating`: an existing metadata row gets `updated_at = created_at`.
