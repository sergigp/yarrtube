## Why

Downloaded videos currently carry no thumbnail: the video player shows nothing
until playback starts, and there is no artwork on disk for a future SPA
video-list view or for media servers (e.g. Plex) mounted on the same output
directory to display as a poster. `yt-dlp` can both embed a cover image in the
video container and write a separate thumbnail file, at negligible extra cost
per download, closing this gap without any new infrastructure.

## What Changes

- Every video download embeds its thumbnail into the saved video file
  (`--embed-thumbnail`), so the file itself carries cover art independent of
  any sibling file.
- Every video download also writes a converted `.jpg` thumbnail next to the
  video file, sharing the exact same base filename as the video
  (`--write-thumbnail --convert-thumbnails jpg`) — the same convention media
  servers like Plex recognize as a video's poster art without listing it as a
  separate library item.
- The video's thumbnail filename is recorded as a fact on the `Video` domain
  entity (mirroring how `filename`/`quality` are already recorded), verified
  against disk rather than assumed, and cleared on reset/redownload.
- Filesystem reconciliation (`playlist-reconciliation`) is updated to treat a
  `Downloaded` video's recorded thumbnail file the same way it already treats
  its recorded video file: not an orphan to be swept.
- Video file cleanup (`video-cleanup`) is updated to also delete a video's
  thumbnail file when its video file is deleted.
- The video listing HTTP response (`video-listing`) includes the recorded
  thumbnail filename, when present, alongside the existing filename.
- Out of scope: no SPA UI changes (no card/list view is built here — this is
  groundwork so a future view can consume the thumbnail), and no backfill for
  videos already downloaded before this change (they simply have no
  thumbnail until they're naturally redownloaded for an unrelated reason).

## Capabilities

### New Capabilities
- `video-thumbnails`: downloading a video also embeds a thumbnail in the
  video file and writes a converted `.jpg` thumbnail alongside it under the
  same base filename.

### Modified Capabilities
- `video-download`: adds a "Downloaded Thumbnail Is Recorded" requirement
  mirroring the existing "Downloaded Filename Is Recorded" one.
- `video-listing`: the videos-for-playlist/videos-for-channel responses
  include each video's recorded thumbnail filename, when present.
- `playlist-reconciliation`: filesystem reconciliation no longer treats a
  `Downloaded` video's recorded thumbnail file as an orphan.
- `video-cleanup`: deleting a video's file also deletes its recorded
  thumbnail file, if any.

## Impact

- `src/infrastructure/shared/ytdlp.rs`: new thumbnail-related `yt-dlp` flags;
  detecting the thumbnail file yt-dlp actually wrote.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`
  and `src/domain/services/video_downloader.rs`: threading the thumbnail
  filename through the download flow.
- `src/domain/video/video.rs`: new `thumbnail_filename` field, set by
  `mark_downloaded`, cleared by `reset_for_redownload`.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: new nullable
  `thumbnail_filename` column.
- `src/application/http/videos/dto.rs`: `VideoResponse` gains a
  `thumbnail_filename` field.
- `src/domain/services/video_reconciler.rs` and
  `src/domain/services/channel_video_reconciler.rs`: orphan detection
  accounts for recorded thumbnail files.
- `src/domain/services/video_file_deleter.rs`: deletes a video's thumbnail
  file alongside its video file.
- No new HTTP routes: thumbnails are served by the existing `/media`
  `ServeDir` mount. No new dependencies (no image-processing crate needed;
  `yt-dlp` + the already-bundled `ffmpeg` do the conversion/embedding).
