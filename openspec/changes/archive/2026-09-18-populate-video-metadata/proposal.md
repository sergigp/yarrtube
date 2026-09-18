## Why

Every downloaded video's `meta.nfo` sidecar is currently created empty, so Plex's
**Plex NFO Movie** agent has nothing to read: no title, plot, studio, genre,
tags, sort order, or stable ID to persist watched state across rescans. This
change populates that file with real per-video metadata fetched from the
YouTube Data API v3, so the Plex library shows proper cards and keeps watched
state, without any manual tagging.

## What Changes

- Rename the per-video metadata sidecar from `meta.nfo` to `movie.nfo`.
- Add a `YoutubeMetadataRepository` port (and `YoutubeApiMetadataRepository`
  adapter) that fetches `videos.list?part=snippet` for a video, separate from
  the existing `YoutubeVideoRepository` used by custom-playlist video adding.
- Add a `VideoMetadata` domain entity holding everything the NFO needs:
  title, plot, studio, director, premiered, year, genre, tags, uniqueid,
  thumb, sorttitle.
- Add a `VideoMetadataRepository` port with a single composed implementation
  that, on `save`, writes `movie.nfo` to the video's folder first and only
  then persists a database row recording that this video's metadata is
  populated; `find` reads only that database row, never the file. The
  database row is the sole source of truth for "has this video's metadata
  been generated."
- After a video download succeeds, fetch its YouTube metadata, resolve its
  `<sorttitle>`, build its `VideoMetadata`, and save it. If the YouTube
  metadata fetch fails for any reason (network error, quota, video gone
  private/deleted), skip the save entirely — no file is written and no
  database row is created — and the video is still marked `Downloaded`
  (metadata generation never fails or retries the download itself).
- Add a hardcoded YouTube `categoryId` → genre-name table; an unmapped
  category ID omits `<genre>` rather than failing.
- Resolve `<sorttitle>` as a zero-padded numeric prefix from
  `PlaylistVideo.position` when the video came from a YouTube-linked
  playlist and has one; otherwise (custom playlists, and all
  channel-tracked videos — a channel's recorded position is a recency rank
  that shifts on every reconcile pass, not a stable order) a zero-padded
  prefix derived from the video's YouTube publish date.
- Truncate `<plot>` to 500 characters, cut at the last word boundary, with a
  trailing ellipsis when truncated.
- Add a reconcile-time repair pass, for both playlist and channel
  reconciliation, that regenerates metadata for any `Downloaded` video with
  no `VideoMetadataRepository` row — self-healing a failed or skipped
  generation on the next reconcile pass without re-downloading the video
  file itself.
- Add an XML-serialization dependency (e.g. `quick-xml`) so `movie.nfo` is
  written with correct escaping rather than string concatenation.
- Stop writing the empty placeholder file inside `ytdlp::download_video`;
  `VideoDownloaderRepository`/`ytdlp.rs` stays a pure `yt-dlp`
  process-invocation layer with no YouTube API or XML knowledge.

**Out of scope**: triggering a Plex library scan after a download batch;
any migration or backfill for videos already downloaded under the previous
blank-`meta.nfo` layout.

## Capabilities

### New Capabilities
- `video-metadata`: fetching a video's YouTube metadata, mapping it (plus a
  locally-resolved sort order) into `movie.nfo` content, and persisting a
  completeness record for it.

### Modified Capabilities
- `video-download`: the "Metadata File Placeholder" requirement (empty
  `meta.nfo` on every successful download) is superseded — see
  `video-metadata`.
- `playlist-reconciliation`: gains a reconcile-time pass that regenerates a
  `Downloaded` video's metadata when it has none recorded.
- `channel-video-sync`: gains the same reconcile-time metadata-repair pass
  for channel-tracked videos.

## Impact

- New Cargo dependency for XML serialization.
- New SQLite table for recorded video metadata (embedded `CREATE TABLE IF
  NOT EXISTS`, matching this repo's existing per-repository table-ownership
  pattern — no separate migrations directory).
- `YOUTUBE_API_KEY` (already configured) is reused by a second repository
  construction in `serve.rs`; `YoutubeVideoRepository` itself is untouched.
- `src/infrastructure/shared/ytdlp.rs`: removes the blank-file write.
- `src/domain/services/video_downloader.rs`: gains metadata-generation
  orchestration after a successful download.
- `src/domain/services/video_reconciler.rs` and
  `src/domain/services/channel_video_reconciler.rs`: gain a metadata-repair
  loop and new constructor dependencies.
