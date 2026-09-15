## Why

Channels are trackable today (`channel-crud`) but nothing ever populates them —
there is no way to discover, download, or clean up a channel's videos. We want
channel video sync to work the same way playlist sync already does (recurring
reconcile, on-demand force-reconcile, automatic download on discovery,
automatic file cleanup on removal), capped to the channel's most recent
`video_limit` uploads.

Doing that cleanly requires first decoupling `Video` from `Playlist`: today a
video's download state (`status`, `quality`, `filename`) is stored as a single
row keyed by `(playlist_id, video_id)`, which has no room for a video also
being tracked by a channel. Splitting the "membership in a container" concern
from the "video is a downloaded file" concern lets both playlists and channels
own videos without either aggregate knowing about the other, and without
duplicating the task/event machinery that already drives downloads and
cleanup.

## What Changes

- **BREAKING**: `Video` is decoupled from `Playlist`. It gains its own
  surrogate id and no longer carries `playlist_id`. A new `PlaylistVideo`
  entity (own id, repository, table) records that a `Video` belongs to a
  `Playlist`, with its own `position`. A `Video` row belongs to exactly one
  owning relation (one `PlaylistVideo` or one `ChannelVideo`) — the same
  YouTube video tracked by two containers gets two independent `Video` rows,
  each with its own download state, matching today's behavior.
- **BREAKING**: new `ChannelVideo` entity/repository (own id, `channel_id`,
  `video_id`, `position`), the channel-side mirror of `PlaylistVideo`.
- **BREAKING**: `DomainEvent::VideoAdded`/`VideoDeleted` are replaced by
  per-container events: `VideoAddedToPlaylist`/`VideoRemovedFromPlaylist` and
  new `VideoAddedToChannel`/`VideoRemovedFromChannel`. `Task::DownloadVideo`
  and `Task::DeleteVideoFile` stay single, container-agnostic task types —
  each subscriber resolves its container's `quality`/output directory before
  scheduling, so the task payload only ever carries already-resolved
  primitives (video id, quality, output directory, filename).
- **BREAKING**: SQLite schema is rebuilt around the new entities; the one
  existing `ALTER TABLE` (the `position` column bolt-on) and any future
  ad-hoc migrations are replaced by schemas that just declare their final
  shape in `CREATE TABLE`. No migration path from an existing database —
  existing local databases must be dropped and recreated.
- Add `Channel.path`, required at creation, the same way `Playlist.path`
  works today, so channel downloads know an output directory.
- Add channel video discovery via `yt-dlp --flat-playlist --print-json`
  against the channel's `/videos` tab, capped to the channel's `video_limit`
  most recent uploads (JSON output, not delimited text, so a title
  containing `|` can't corrupt parsing).
- Add recurring + on-demand (`POST /channels/{handle}/reconcile`) channel
  reconciliation, mirroring `playlist-reconciliation`: diffs the channel's
  current top-N videos against stored `ChannelVideo` rows, adds newly-seen
  videos, and evicts (deletes + publishes `VideoRemovedFromChannel` for) any
  previously-tracked video that has aged out of the top N.
- Video download/cleanup/listing/playback now apply uniformly to
  channel-owned videos, not just playlist-owned ones.

## Capabilities

### New Capabilities
- `channel-video-sync`: recurring and on-demand reconciliation of a tracked
  channel's most recent `video_limit` uploads against stored `ChannelVideo`
  records, via `yt-dlp`, publishing `VideoAddedToChannel`/
  `VideoRemovedFromChannel`.

### Modified Capabilities
- `channel-crud`: channel creation requires and persists a `path`.
- `playlist-reconciliation`: publishes `VideoAddedToPlaylist`/
  `VideoRemovedFromPlaylist` in place of `VideoAdded`/`VideoDeleted`.
- `video-download`: download is also triggered by a video being added to a
  tracked channel, not only a tracked playlist.
- `video-cleanup`: file deletion is also triggered by a video being removed
  from a tracked channel, not only a tracked playlist.
- `video-listing`: add an endpoint to list a tracked channel's videos.
- `video-playback`: a downloaded video's media URL is also served when it
  belongs to a tracked channel, not only a tracked playlist.

## Impact

- Domain: `domain/video` (drop `playlist_id`, add surrogate id), new
  `domain/playlist_video` and `domain/channel_video` (or equivalent
  aggregate placement), `Channel` gains `path`, reconcile orchestration in
  the domain layer creates/deletes across the `Video` + owning-relation
  repositories explicitly rather than a repository hiding the join.
- Infrastructure: new `SqlitePlaylistVideoRepository`,
  `SqliteChannelVideoRepository`, a `SqliteVideoRepository` rewritten around
  the surrogate id, a new yt-dlp-backed channel-videos repository, and every
  `CREATE TABLE` statement rewritten to its final shape (no `ALTER TABLE`
  anywhere in the codebase).
- Application: new `Task::ReconcileChannel` and its handler; new
  `VideoAddedToChannel`/`VideoRemovedFromChannel` subscribers alongside
  renamed `VideoAddedToPlaylist`/`VideoRemovedFromPlaylist` ones;
  `VideoDownloader`/`VideoFileDeleter` lose their `PlaylistRepository`
  dependency; new HTTP routes for channel force-reconcile and channel video
  listing; `create_channel` gains a required `path` field.
- Data: existing local SQLite databases are not migrated and must be
  recreated from scratch.
