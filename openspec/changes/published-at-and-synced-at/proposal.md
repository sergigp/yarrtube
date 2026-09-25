## Why

We can't tell when a video was published on YouTube or when we downloaded it.
The publish timestamp is fetched from the YouTube Data API but only kept as
the NFO strings `premiered` (`YYYY-MM-DD`) and `year`. `videos.updated_at`
isn't a sync time either: thumbnail fetches, redownload resets and error
transitions also bump it. Separately, the metadata upsert overwrites
`created_at` on every re-save (e.g. after a redownload).

## What Changes

- Add a nullable `synced_at` to `videos`: set when a download succeeds, cleared
  when the video is reset for redownload. Backfilled from `updated_at` for rows
  already `downloaded`. `updated_at` keeps its current meaning (row last touched).
- Expose `synced_at` on the per-playlist and per-channel video listing responses.
- Add `published_at` (full timestamp from the API's `publishedAt`) to video
  metadata. Backfilled from `premiered` at midnight UTC.
- **BREAKING** (storage only): drop the `premiered` and `year` columns from
  `video_metadata`. The NFO writer derives both from `published_at`; the
  `movie.nfo` content does not change.
- Add `updated_at` to video metadata, and keep `created_at` from being
  overwritten when metadata is re-saved. Both come from the domain clock.
- Out of scope: sorting the recent videos list (it stays on `created_at`, the
  moment the video was wanted) and exposing metadata in the video DTO (next
  change).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `video-download`: a successful download records its sync time on the video;
  a redownload reset clears it.
- `video-metadata`: records the video's publish timestamp and when the
  metadata was first generated and last regenerated.
- `video-listing`: playlist and channel video listings include each video's
  sync time.

## Impact

- New migration `migrations/0003_published_at_and_synced_at.sql`, registered in
  `sqlite_migrations.rs`.
- Domain: `Video` (`synced_at`), `VideoMetadata` (`published_at`, `created_at`,
  `updated_at`; `premiered`/`year` removed), `mapping.rs`, `nfo.rs`.
- Services that build or save metadata (`VideoDownloader`, playlist/channel
  reconcilers) pass the clock's `now`.
- Adapters: `SqliteVideoRepository`, `SqliteVideoMetadataRepository`.
- HTTP: `VideoResponse` gains `synced_at`.
