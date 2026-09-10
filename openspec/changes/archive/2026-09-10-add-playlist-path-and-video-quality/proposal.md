## Why

Today a playlist's storage location is implicitly its `name` — `name` is reused as the filesystem folder, so users can't organize downloads (e.g. nested folders) without renaming the playlist itself, and a downloaded video's actual quality is never recorded, making it impossible to later detect that a playlist's videos are stale relative to its current `quality` setting. Decoupling storage location from display name, and recording the quality each video was actually downloaded at, lays the groundwork both features need.

## What Changes

- Add a required `path` property to `Playlist`, supplied on creation, used instead of `name` to build a video's output directory under `YARRTUBE_VIDEOS_PATH`. Supports nested subdirectories (e.g. `a/b/c`), validated against path traversal (no `..`, no absolute paths, no empty segments). **BREAKING**: `POST` create-playlist now requires `path` in the request body.
- Add an optional `quality` property to `Video`, set to the resolution tier it was actually downloaded at whenever a download succeeds (`None` until then).
- Move the `Quality` value object from `domain/playlist/` to `domain/shared/`, since both `Playlist` and `Video` now depend on it.

Out of scope for this change: an update-playlist endpoint, detecting drift between a playlist's `quality` and its videos' recorded `quality`, and retriggering downloads — `Video.quality` only lays the groundwork for a later change to build that on top of.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist-crud`: Create Playlist now requires a `path` (validated, nested-subdirectory-aware) alongside `name` and `quality`; duplicate-ID creation ignores a re-supplied `path` the same way it already ignores a re-supplied `quality`.
- `video-download`: the per-playlist output directory is now built from the playlist's `path` instead of its `name`; a video's actually-downloaded quality tier is recorded on it once its download succeeds.

## Impact

- **Domain**: `src/domain/playlist/playlist.rs` (new `path` field), new `src/domain/playlist/playlist_path.rs` value object, `src/domain/video/video.rs` (new `quality` field, `mark_downloaded` takes the resolved `Quality`), `Quality` relocated from `src/domain/playlist/quality.rs` to `src/domain/shared/`.
- **HTTP**: `src/http/playlists/dto.rs` — `CreatePlaylistRequest` gains required `path`; `PlaylistResponse` gains `path`.
- **Repositories**: `sqlite_playlist_repository.rs` (`playlists.path` column, `NOT NULL`), `sqlite_video_repository.rs` (`videos.quality` column, nullable) — both via the existing `CREATE TABLE IF NOT EXISTS` convention, no migration tooling (matches this repo's established precedent for schema additions).
- **Services**: `src/domain/video/service.rs` — `download_video` and `delete_video_file` build `output_dir` from `playlist.path` instead of `playlist.name`; `download_video` stamps the resolved `Quality` onto the video on success.
- No HTTP/API surface for `Video.quality` — no video endpoints exist yet.
