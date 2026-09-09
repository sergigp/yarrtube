## REMOVED Requirements

### Requirement: CLI Invocation
**Reason**: The standalone CLI `download` command downloaded an ad hoc, untracked playlist with no persisted `Playlist` entity to configure. Quality/format configuration (`add-playlist-download-quality`) only applies to playlists tracked via the `playlist-crud` HTTP API and downloaded through `video-download`'s task-driven flow.
**Migration**: Create the playlist via the `playlist-crud` HTTP API (`POST /playlists`) with a `quality` value; the daemon downloads it automatically per `video-download`.

### Requirement: API Key Configuration
**Reason**: This requirement only concerned the standalone CLI's own `.env` key lookup. The daemon (`serve`) already has its own, independent `YOUTUBE_API_KEY` check that is unaffected by this removal.
**Migration**: No action needed — the daemon's `YOUTUBE_API_KEY` configuration continues to apply unchanged.

### Requirement: Playlist Resolution
**Reason**: Playlist resolution for tracked playlists is already covered by `playlist-sync`; this requirement duplicated that logic for the now-removed ad hoc CLI path.
**Migration**: Track the playlist via `playlist-crud`; `playlist-sync` resolves it and keeps its videos up to date automatically.

### Requirement: Sequential Video Download
**Reason**: One-at-a-time CLI downloading is superseded by `video-download`'s per-video task-driven downloads, which run through the task queue instead.
**Migration**: No direct equivalent needed — tracked-playlist downloads already run through `video-download`.

### Requirement: Per-Video Failure Isolation
**Reason**: Failure isolation for the removed CLI path is superseded by `video-download`'s per-video status tracking (pending/in-progress/downloaded/errored) and `task-scheduling`'s retry/dead-letter mechanism, which are stronger guarantees than the CLI's printed summary.
**Migration**: Inspect a video's status via the existing playlist/video tracking instead of a CLI-printed summary.
