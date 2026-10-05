## Why

A video dropped halfway stays in "Continue watching" forever: there is no way to mark a single video watched. Marking its whole channel watched doesn't reliably help either. If the video is loaded in the player, the player later reports its old position. The rewatch rule (watched and past 10%) then un-watches the video again, so it comes back to "Continue watching" and the channel badge shows 1 again.

## What Changes

- New endpoint to mark a single video watched, applied to every stored copy of that YouTube video. It changes nothing when the video is already watched, and it leaves `last_played_at` alone.
- Progress reports carry `was_watched`: the watched state the client believed when its playback session started. The daemon ignores a report that is stale, meaning the client believed the video unwatched but it is now watched. For such a report it changes nothing and answers `watched: true`. **BREAKING** (API): `was_watched` is required on `POST /api/videos/{id}/progress`.
- The player's progress tracking starts a new playback session when the selected video's watched state changes under it. It drops the held position, pauses, and seeks to 0, so a stale position is never re-sent.
- A vertical "⋮" menu with a "Mark as watched" item appears on:
  - every home card
  - every row of the channel and playlist video lists
  - the title row of the video detail pane under the player

  The item is disabled when the video is already watched or not downloaded.
- Out of scope: marking a video unwatched, "Mark all watched" for playlists, auto-advancing after a mark.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `watch-state`:
  - add "Mark A Video Watched"
  - progress reports carry the client's session watched state, and stale reports are ignored
- `web-ui`:
  - per-video "Mark as watched" menu on cards, list rows and the video detail pane
  - the player starts a new progress session when the selected video's watched state changes, and reports that state with each report

## Impact

- Backend:
  - `src/domain/video/video.rs` (`update_watch_state` stale guard)
  - `src/domain/services/video_watch_state_updater.rs` (new `mark_video_watched`, `update` takes `was_watched`)
  - `src/application/http/videos/` (new handler, `was_watched` on the request DTO)
  - `src/application/http/mod.rs` (route)
- Frontend:
  - `web/src/api/` (client and types)
  - `web/src/hooks/useWatchProgress.ts`
  - new `VideoActionsMenu` component
  - `Home.tsx`, `VideoListPane.tsx` (row split into a select button plus a sibling menu trigger), `VideoDetail.tsx`, `ChannelDetail.tsx`, `PlaylistDetail.tsx`
- API: `POST /api/videos/{id}/watched` (new), `POST /api/videos/{id}/progress` (new required field).
