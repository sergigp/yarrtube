## Why

The SPA has no landing view — it opens straight into the Playlists tab with no way to see what's been recently synced across every tracked playlist and channel at a glance. A home page showing the last N synced videos, newest first, gives an at-a-glance overview and a fast path to resume watching.

## What Changes

- Add `VideoSearcher.list_recent(limit)`, a cross-aggregate read that unions videos tracked by playlists and channels, filtered to `status = DOWNLOADED`, sorted by `videos.created_at` descending (sync time — no YouTube upload date is captured anywhere today), truncated to `limit`.
- Add `GET /api/videos/recent?limit=N` (default 20, capped at 100) returning each video's `id` (youtube_id), `title`, and `source` (`kind`: `playlist` | `channel`, `id`: playlist id or channel handle). No dedup: the same YouTube video tracked by two sources appears once per source.
- Add a new **Home** tab in the SPA, first/default, listing these videos as title-only cards. Clicking a card switches to the Playlists or Channels tab (per `source.kind`), resolves the full playlist/channel via the existing list endpoints, and preselects + autoplays the clicked video in `PlaylistDetail`/`ChannelDetail`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `video-listing`: adds a third requirement, "List Recently Synced Videos Across Sources," alongside the existing per-playlist and per-channel listing requirements.

## Impact

- Backend: new `VideoSearcher` method, new HTTP handler + route (`src/application/http/videos/`, `src/application/http/mod.rs`), new response DTO.
- Frontend: new `Home` tab component, `App.jsx` tab wiring, `api.js` new `fetchRecentVideos()` call, and lifting enough state into `PlaylistsTab`/`ChannelsTab` to accept a deep-link target (playlist/channel id + video id to preselect).
- No schema/migration changes — reuses the existing `videos`, `playlist_video`, and `channel_video` tables.
