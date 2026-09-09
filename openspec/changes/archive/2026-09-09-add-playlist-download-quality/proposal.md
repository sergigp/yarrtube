## Why

Downloads currently run bare `yt-dlp <url>` with no format/quality selection, so the container/codec and resolution of a downloaded video are whatever YouTube's top-ranked streams happen to be for that video (often VP9/AV1 + Opus in a `.webm` container), which plays inconsistently across devices and gives no control over per-playlist storage/bandwidth cost. The event-driven download flow (`video-download`) already downloads through a stored, per-playlist entity, making the playlist the natural place to configure this per playlist rather than instance-wide.

## What Changes

- Add a required `quality: high | mid | low` field to playlist creation. It cannot be changed after creation (no update endpoint exists for playlists today, and this does not add one).
- Downloads always target a fixed, opinionated mp4 + h264(avc1) + aac output for maximum device/player compatibility, regardless of tier. `quality` only controls the resolution cap: `high` = uncapped, `mid` = ≤720p, `low` = ≤480p. The mp4/h264/aac preference degrades gracefully (falls back to the best available stream) rather than hard-failing a video that has no compatible stream at the requested resolution.
- `quality` is resolved from the `Playlist` when `DownloadVideoOnVideoAdded` schedules the `download_video` task, and is carried in that task's payload as part of its download intent — **BREAKING**: `Task::DownloadVideo`'s payload shape changes from `{playlist_id, video_id}` to `{playlist_id, video_id, quality}`.
- **BREAKING/Removal**: The standalone CLI `download` command (`yarrtube download <playlist_id> <output_path>`), `Commands::Download`, and `YtDlpDownloaderClient`/`YoutubeDownloaderClient` are removed entirely. It downloaded ad hoc, untracked playlists with no `Playlist` entity to read `quality` from, and is superseded by the daemon's tracked-playlist/task-driven download flow.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `playlist-crud`: playlist creation requires a `quality` value (`high`/`mid`/`low`); invalid or missing values are rejected the same way an invalid name is today.
- `video-download`: downloads apply the owning playlist's `quality` (resolution cap, fixed mp4/h264/aac preference) instead of relying on `yt-dlp`'s own defaults; `quality` is resolved once, when the download task is scheduled, and travels with that task rather than being re-read from the playlist at execution time.
- `playlist-download`: **removed**. The standalone CLI download-a-playlist-ad-hoc capability is retired; every requirement in this capability is dropped with no replacement in this capability's scope (its function is fully covered by `video-download`).

## Impact

- **Domain**: `Playlist` (new `quality` value object/field), `Task::DownloadVideo` payload, `VideoDownloaderRepository::download` signature.
- **Subscribers**: `DownloadVideoOnVideoAdded` gains a `PlaylistRepository` dependency.
- **Services**: `VideoService::download_video` takes `quality` as a parameter instead of implicitly trusting the playlist's current value.
- **Infrastructure**: `infrastructure/shared/ytdlp.rs::download_video` gains yt-dlp args derived from `quality`; `infrastructure/client/youtube_downloader_client.rs` (`YtDlpDownloaderClient`) and `infrastructure/repositories/youtube_video_downloader_repository.rs`'s CLI-only bits are removed.
- **CLI**: `cli::download_command`, `Commands::Download` removed from `main.rs`/`cli/mod.rs`.
- **HTTP**: playlist creation endpoint's request/response payload gains `quality`.
- **Storage**: `playlists` table gains a `quality` column (the `CREATE TABLE IF NOT EXISTS` pattern used today does not retrofit this onto an already-existing database file — acceptable pre-1.0, called out in design.md).
