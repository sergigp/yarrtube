## Why

Custom playlists were removed in #41, so every playlist is now YouTube-linked
and every playlist video has a position in its source YouTube playlist (prod
check: 0 of 1197 `playlist_videos` rows have a `NULL` position). The schema,
`PlaylistVideo.position: Option<i64>` and several spec scenarios still model a
video with no position. That makes `None` handling spread through the code
and the specs describe behaviour that can no longer happen.

## What Changes

- `playlist_videos.position` becomes `NOT NULL` (table-rebuild migration).
- `PlaylistVideo.position` becomes `i64`; the position-less constructor is
  removed and `create_with_position` becomes the only constructor.
- Metadata `sorttitle` for a playlist video always uses its playlist position;
  publish date stays for channel videos only.
- Specs drop every custom-playlist scenario and the "no position" case.
- **BREAKING** (data): the migration fails if any `playlist_videos` row has a
  `NULL` position. Prod has none.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `playlist-reconciliation`: restates creation-triggered and on-demand reconciliation without their custom-playlist scenarios (REMOVED + renamed ADDED, since a MODIFIED block cannot drop scenarios), and removes the "membership diff applies only to YouTube-linked playlists" requirement.
- `video-metadata`: `sorttitle` requirement restated (REMOVED + renamed ADDED): playlist position for every playlist video, publish date for channel videos only.
- `video-listing`: drops the "custom playlist has no guaranteed order" clause.

## Impact

- `migrations/0006_playlist_video_position_required.sql` (new) and `src/infrastructure/shared/sqlite_migrations.rs`.
- `src/domain/playlist_video/playlist_video.rs`, `src/infrastructure/repositories/sqlite_playlist_video_repository.rs`.
- `src/domain/services/playlist_video_reconciler.rs`, `src/domain/services/video_downloader.rs`: position readers lose their `Option` handling.
- Test seeds that call `PlaylistVideo::create(..)` without a position.
- Out of scope: removing `PlaylistKind` / `playlists.kind` (it now has a single variant). That's a separate cleanup.
