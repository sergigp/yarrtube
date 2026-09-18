## Why

Downloaded videos and their thumbnails currently sit as flat sibling files directly inside each playlist's or channel's output directory. That makes it impossible to attach further per-video artifacts (the `meta.nfo` metadata file this change introduces, and richer metadata in future changes) without either polluting the shared container directory or inventing an ad-hoc naming convention to associate files with a video. Giving every video its own folder solves this cleanly and matches how media servers/managers (Kodi, Jellyfin, etc.) expect a library to be laid out.

## What Changes

- A downloaded video's file, its thumbnail, and a new empty `meta.nfo` placeholder file are now saved together inside a dedicated per-video folder (named from the same sanitized-title logic already used for filenames), instead of as flat sibling files directly in the playlist's/channel's output directory.
- Folder-name collisions (two videos in the same container whose titles sanitize to the same name) are resolved the same way filename collisions are resolved today: the video's YouTube ID is appended to the folder name.
- `Video.filename`/`Video.thumbnail_filename` keep their existing type and meaning (a path relative to the container's output directory) but may now contain one `/` separator pointing into the video's own folder; no database schema change.
- Video deletion (single-video removal, and reconciliation's orphan cleanup) now removes the whole per-video folder rather than trying to delete two named files.
- Filesystem reconciliation's health check for a `Downloaded` video is updated to check the recorded file's existence directly instead of relying on a flat top-level directory listing, since that listing can no longer contain nested paths.
- The SPA's media URL builder is fixed to percent-encode a nested filename segment-by-segment (mirroring how it already treats the container's storage path), so playback keeps working unchanged for the person using the app.
- **Not in scope**: migrating videos already downloaded under the old flat layout. They keep playing exactly as before; only new downloads (and any video reset for redownload by reconciliation) land in the new per-video-folder layout. Both layouts coexist permanently within the same container directory. `meta.nfo`'s content stays empty — populating it is left to a future change.

## Capabilities

### New Capabilities

(none — this reshapes how existing capabilities implement their storage requirements)

### Modified Capabilities

- `video-download`: replaces "Per-Playlist Output Directory" with a per-video-folder layout, and adds a new requirement that an empty `meta.nfo` placeholder file is created alongside each downloaded video.
- `video-naming`: the "Collision Fallback Appends Video ID" requirement changes from resolving a *filename* collision within the container directory to resolving a *folder name* collision within it.
- `video-thumbnails`: the "Thumbnail File Written Alongside Video" requirement's directory reference is updated to be explicit that "the video's output directory" is now that video's own dedicated folder.

## Impact

- **Backend (Rust)**: `src/infrastructure/shared/ytdlp.rs` (output directory/template resolution, folder-collision resolution, `meta.nfo` creation), `src/domain/services/video_downloader.rs` (composes the stored `filename`/`thumbnail_filename` paths), a new small pure helper under `src/domain/video/` for deriving the top-level container entry a stored path belongs to, `src/infrastructure/repositories/filesystem_video_file_repository.rs` (directory-aware deletion, new existence check), `src/domain/services/video_file_deleter.rs`, `src/domain/services/video_reconciler.rs`, `src/domain/services/channel_video_reconciler.rs`.
- **Frontend**: `web/src/api.js` (`videoMediaUrl` encoding fix only).
- **No database migration.** No HTTP API/DTO shape change. No SPA behavior change visible to the user.
