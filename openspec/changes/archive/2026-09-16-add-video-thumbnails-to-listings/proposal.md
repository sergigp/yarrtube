## Why

Videos have carried a downloaded thumbnail file since the `video-thumbnails`
change, but no HTTP response or SPA view renders it anywhere yet — the SPA
only uses it as the big `<video>` player's `poster`. The recent-videos feed
and the per-playlist/per-channel video lists are the natural place for it:
a visual thumbnail lets someone recognize a video at a glance instead of
scanning titles.

## What Changes

- `GET /videos/recent` gains `thumbnail_filename` on each returned video, and
  its `source` object gains a `path` field (the tracking playlist's or
  channel's storage path) so the SPA can build a media URL for it without an
  extra lookup per video — mirroring how `path` is already used everywhere
  else to build media URLs.
- `GET /playlists/{id}/videos` and `GET /channels/{handle}/videos` already
  return `thumbnail_filename` (from the `video-thumbnails` change); no
  backend change needed there.
- The SPA's `Home` tab cards render a thumbnail `<img>` beside the title. The
  `PlaylistDetail`/`ChannelDetail` sidebar video list rows do the same. A
  video with no recorded thumbnail (predates the thumbnails feature, or
  hasn't finished downloading) shows a blank placeholder box of the same
  size instead of a broken image, so rows stay visually aligned.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `video-listing`: the "List Recently Synced Videos Across Sources"
  requirement's response gains `thumbnail_filename` per video and `path` on
  `source`.

## Impact

- Backend: `src/domain/video/recent_video.rs` (`VideoSource` carries each
  source's path), `src/domain/services/video_searcher.rs` (threads the
  path through from the already-fetched `Playlist`/`Channel`),
  `src/application/http/videos/dto.rs` (`RecentVideoResponse` gains
  `thumbnail_filename`, `RecentVideoSourceResponse` gains `path`).
- Frontend: `web/src/components/Home.jsx`, `web/src/components/
  PlaylistDetail.jsx`, `web/src/components/ChannelDetail.jsx` render a
  thumbnail `<img>` (or placeholder) per video row; new CSS in
  `web/src/App.css` for the thumbnail/placeholder box.
- No schema/migration changes — `thumbnail_filename` and container `path`
  are already recorded; this only surfaces them further.
