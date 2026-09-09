## Context

See proposal.md - Why/What Changes for motivation and scope. Relevant current state:

- `download_video` (`infrastructure/shared/ytdlp.rs`) runs bare `Command::new("yt-dlp").arg(video_url)` — no format flags at all, shared by both the CLI path (being removed) and the event-driven path.
- `VideoService::download_video(playlist_id, video_id, is_last_attempt)` already calls `playlist_repository.find(&playlist_id)` to build the per-playlist output directory, then calls `VideoDownloaderRepository::download(url, output_dir)`.
- `DownloadVideoOnVideoAdded` schedules `Task::DownloadVideo { playlist_id, video_id }` purely from the `VideoAdded` event's payload — it has no `PlaylistRepository` dependency today.
- `playlists` is created via `CREATE TABLE IF NOT EXISTS` in `sqlite_playlist_repository.rs`, run unconditionally at startup.

## Goals / Non-Goals

**Goals:**
- Resolve `quality` once, when the download task is scheduled, and carry it as part of that task's payload/intent.
- Keep container/codec preference (mp4/h264/aac) fixed and out of user configuration; only the resolution cap varies by tier.
- Keep the mp4/h264/aac preference soft (never hard-fails a video solely because no such stream exists).

**Non-Goals:**
- No update endpoint for playlists (quality is set once, at creation).
- No per-tier container/codec choice (e.g. mkv for `high`) — out of scope per proposal.
- No migration tooling for existing SQLite files missing the new `quality` column (see Risks).

## Decisions

**yt-dlp selector shape per tier** — combine a hard `-f` resolution filter (predictable file size/bandwidth per tier) with a soft `-S "ext:mp4:m4a"` sort key (graceful fallback instead of a hard filter on container/codec):
- `high`: `-f "bv*+ba/b" -S "ext:mp4:m4a"` (no height bound)
- `mid`: `-f "bv*[height<=720]+ba/b[height<=720]" -S "ext:mp4:m4a"`
- `low`: `-f "bv*[height<=480]+ba/b[height<=480]" -S "ext:mp4:m4a"`
- always: `--merge-output-format mp4`

Alternative considered: a hard `-f` filter on `ext=mp4`/`ext=m4a` too (e.g. `bv*[height<=720][ext=mp4]+ba[ext=m4a]/b[height<=720][ext=mp4]`). Rejected: this can produce zero matches for a video with no native mp4 stream at that resolution and fail the download outright, which then burns through `task-scheduling`'s 5 retries and dead-letters a video that a softer preference would have downloaded successfully (just not in mp4).

**Quality resolved in the subscriber, not the domain event** — `DownloadVideoOnVideoAdded` gains a `PlaylistRepository` dependency and looks up `playlist.quality` when it schedules `Task::DownloadVideo`, embedding it in the task's payload: `{playlist_id, video_id, quality}`. `DomainEvent::VideoAdded`'s payload is unchanged.

Alternative considered: resolve `quality` earlier, in `VideoService::sync_playlist_videos` (which already fetches `Playlist` to check existence, then discards it), and add `quality` to `VideoAdded` itself, with the subscriber just forwarding it. Rejected: `VideoAdded` is a fact about video discovery; folding download configuration into it conflates the two, and changes a persisted/dead-letterable event's shape for no behavioral gain (quality is immutable post-creation, so there's no staleness risk either way — this is purely about which layer owns the lookup).

**`VideoService::download_video` takes `quality` as a parameter** rather than re-reading `playlist.quality` at execution time, even though it already looks up `Playlist` for the output directory. This makes the task's payload authoritative for "what to download it as," matching the task-payload-as-intent decision above; the playlist lookup that remains is solely for `playlist.name` (output directory).

**Container/codec is fixed, not configurable** — mp4/h264/aac chosen for near-universal device/player compatibility (browsers, iOS/Android native players, Plex/Jellyfin direct play, smart TVs, QuickTime) over the better compression of VP9/AV1+webm, which plays inconsistently on non-browser/non-VLC targets. Not exposed as a setting because there's one clearly-right default for this use case (videos saved for general playback, not archival-grade fidelity).

## Risks / Trade-offs

- [`quality` on `playlists` retrofits nothing onto an already-existing database file, since it's created via `CREATE TABLE IF NOT EXISTS`] → Acceptable: this is a pre-1.0 project with no real deployments yet. No migration path is provided; a stale `playlists` table missing the column is a startup-time schema error, not a silent data-loss risk.
- [`Task::DownloadVideo`'s payload shape changes from `{playlist_id, video_id}` to `{playlist_id, video_id, quality}`, breaking `Task::decode_download_video_payload`'s old shape] → No backward-compatibility shim: any already-scheduled/dead-lettered `download_video` task in the old shape would fail to decode. Acceptable for the same pre-1.0 reason above.
- [Capping resolution via a hard `-f` filter still risks zero-match videos if a video has literally no stream at or below the cap (rare, but possible for very old/low-quality-source uploads)] → yt-dlp's `-f` selector with the `/b[height<=N]` fallback branch already covers this: if no split video+audio combination matches, it falls back to a single combined format at or under the cap; only a video with *no* stream at all under the cap would still fail, which is an accurate failure (nothing to download at that resolution).
