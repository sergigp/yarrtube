## Why

A video's thumbnail image doesn't exist anywhere — on disk or in the database — until its entire video file has finished downloading, because today's only thumbnail-producing step is the `--write-thumbnail`/`--convert-thumbnails` flags on the same `yt-dlp` invocation that pulls the video body. With a single serial task executor and an hourly reconcile interval, a newly added video can sit `PENDING` for a long time — behind other queued downloads — showing its title with no artwork at all in the UI. Fetching just the thumbnail image, independently and ahead of the full video download, gives the UI something to show immediately.

## What Changes

- As soon as a video's record is created (YouTube-linked playlist sync, channel sync, or a video added to a custom playlist), the system attempts a lightweight, best-effort thumbnail-only fetch (`yt-dlp --skip-download --write-thumbnail --convert-thumbnails jpg`) before announcing the video via its `VideoAdded(ToPlaylist/ToChannel)` event. A failure here never blocks video creation or the eventual full download — it's logged and left for reconcile to retry.
- That fetch decides and creates the video's own per-video output folder up front. The real download, when it later runs, reuses that exact folder (detected via the video's already-recorded `thumbnail_filename`) instead of independently resolving a folder-name collision, so the two steps never fight over naming the same video's folder.
- Reconcile's orphan-sweep is extended to protect a not-yet-downloaded video's pre-fetched thumbnail folder (previously only a `Downloaded` video's folder was protected), and gains a new self-heal pass — mirroring the existing missing-metadata recovery — that retries the thumbnail fetch for any video that still has none, regardless of status.
- No new `Task` type, no new `DomainEvent` variant, no database schema change: the fetch runs inline at video-creation time, and the already-decided folder is recovered from the existing `thumbnail_filename` field rather than a new column.

## Capabilities

### New Capabilities

(none — this adds behavior to how existing capabilities create and store videos)

### Modified Capabilities

- `video-thumbnails`: adds a new requirement that a thumbnail is fetched independently, ahead of the full download, as soon as a video is created.
- `video-download`: the "Per-Playlist Output Directory" requirement is updated so a video whose thumbnail was already fetched reuses that same per-video folder for its full download instead of re-resolving a naming collision.
- `playlist-reconciliation`: video persistence for a YouTube-linked playlist now attempts a thumbnail fetch before publishing `VideoAddedToPlaylist`; the orphan sweep protects a pending video's pre-fetched folder; a new missing-thumbnail recovery pass is added.
- `channel-video-sync`: the same three changes as `playlist-reconciliation`, for channel-tracked videos.
- `custom-playlist-crud`: adding a video to a custom playlist now attempts a thumbnail fetch before publishing its `VideoAdded` event.

## Impact

- **Backend (Rust)**: `src/infrastructure/shared/ytdlp.rs` (new thumbnail-only `yt-dlp` invocation, exposing folder-collision resolution for reuse), `src/infrastructure/repositories/youtube_video_downloader_repository.rs` (new port method), `src/domain/video/video.rs` (new `with_thumbnail` transition), `src/domain/services/video_downloader.rs` (reuse a pre-decided folder when present), a new small domain service for the thumbnail-only fetch, `src/domain/services/video_reconciler.rs` and `src/domain/services/channel_video_reconciler.rs` (call the new fetch on video creation, widen orphan-sweep protection, add missing-thumbnail recovery), `src/domain/services/custom_playlist_video_adder.rs` (call the new fetch on video creation; needs the videos-root path injected).
- **No database migration.** No HTTP API/DTO shape change. No SPA change — `Thumbnail.jsx`/`videoMediaUrl` already render `thumbnail_filename` whenever it's present, regardless of the video's download status.
