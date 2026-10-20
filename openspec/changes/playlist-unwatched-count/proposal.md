# Proposal

## Why

Channels show how many downloaded videos are still unwatched, and playlists don't. That makes playlists used as queues ("watch later", courses) harder to keep up with. Playlists are uncapped and also used for rewatching (music, favorites, archives), where a count would only add noise. So each playlist gets an opt-out.

## What Changes

- `GET /api/playlists` returns each playlist's count of downloaded, unwatched videos, and whether that count is hidden.
- New per-playlist setting `hide_unwatched_count` (default `false`). Existing playlists keep it `false` after upgrade. It changes through `PATCH /api/playlists/:id`, like `exclude_from_home`. It is display-only: the count is still computed and returned.
- Hidden playlists add nothing to category unwatched counts or the categories' unwatched total. A video also reached through a counted member still counts once.
- New `POST /api/playlists/:id/watched` marks every downloaded video of the playlist watched. Since watch state belongs to the YouTube video, this also marks those videos' copies in channels.
- Web UI:
  - Playlist sidebar rows show the unwatched badge unless the count is hidden.
  - The playlist row menu and the playlist page header menu gain "Mark all watched" and "Hide unwatched count" / "Show unwatched count".
  - The playlist page header summary shows the unwatched count.
  - Playlists in the sidebar are ordered the way channels are: unread first, most unwatched first. A playlist with a hidden count sorts as caught up.
  - Badges refresh promptly after playback crosses the watched threshold, now for playlists as well as channels.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist-crud`: listing includes the unwatched count and the hide setting; the update endpoint accepts `hide_unwatched_count`.
- `watch-state`: new "Mark A Playlist Watched" requirement; the mark-video scenario mentions playlist counts too.
- `category-crud`: category unwatched counts and the unwatched total skip videos reached only through hidden playlists.
- `web-ui`: playlist badge, new playlist menu actions (row and page header), playlist ordering, and prompt badge refresh for playlists.

## Impact

- **Backend**:
  - Migration `0012`.
  - `Playlist` entity and SQLite repository.
  - New `PlaylistView` + `PlaylistViewSearcher`, replacing `PlaylistSearcher`.
  - `PlaylistVideoRepository::list`.
  - `PlaylistUpdater`, plus `VideoWatchStateUpdater`, which needs the playlist repositories.
  - `CategoryViewSearcher`, which skips hidden playlists' videos when counting.
  - Playlists HTTP handlers, DTOs and router.
- **Web**:
  - `api/types.ts`, `api/client.ts`, `api/queries.ts` and the test builders.
  - `EntryActionsMenu`, `DetailHeader`, `PlaylistDetail`, `Sidebar`, `lib/sidebarSections.ts` and `useWatchProgress`.
- **Smoke tests**: the playlist spec covers the new `POST /playlists/{id}/watched` route.
