## Why

`GET /playlists/{id}/videos` currently returns a YouTube-linked playlist's videos ordered by their video ID (`ORDER BY video_id ASC` in `list_for_playlist`), which is an opaque, effectively-random string YouTube assigns — not the order the playlist is actually arranged in on YouTube. The playlist's real position is fetched correctly from the YouTube Data API during sync but is never persisted, so it's lost by the time videos are listed back out.

## What Changes

- Parse the `snippet.position` field YouTube's `playlistItems` API already returns for each item (currently unused — only `title` and `resourceId.videoId` are read).
- Persist each video's playlist position alongside its other synced fields.
- Update the stored position for existing videos on every reconcile pass (not just at first discovery), so a manual reorder on YouTube is reflected the next time the playlist syncs.
- Order `GET /playlists/{id}/videos` by stored position instead of video ID, for YouTube-linked playlists.
- Custom playlists (no YouTube playlist behind them) have no position source and keep today's `video_id ASC` ordering — out of scope for this change.
- Downloaded filenames on disk are unaffected — no ordinal prefix is added; this change is scoped to the listing endpoint only.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `video-listing`: videos returned for a YouTube-linked playlist are ordered by playlist position instead of video ID.
- `playlist-reconciliation`: the recurring/on-demand sync pass now also persists each video's current playlist position, for newly-discovered and already-known videos alike.

## Impact

- `src/infrastructure/repositories/youtube_playlist_items_repository.rs`: parse `snippet.position` into `PlaylistVideo`.
- `src/domain/video/video.rs`: add a `position` field to `Video`.
- `src/domain/video/service.rs` (`sync_playlist_membership`): pass position through when creating and updating videos.
- `src/infrastructure/repositories/sqlite_video_repository.rs`: add a `position` column; change `list_for_playlist`'s `ORDER BY` to use it, falling back to `video_id ASC` for rows with no position (Custom playlists).
- `src/http/videos/mod.rs` and its `VideoResponse` DTO: no response shape change expected, only ordering.
