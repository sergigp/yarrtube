## Context

See proposal.md - Why. Today `Video` is a single row keyed by
`(playlist_id, video_id)` that conflates two concerns: "this YouTube video is
a member of this container" and "this is the on-disk download state of that
membership" (`status`, `quality`, `filename`). `VideoReconciler`,
`VideoDownloader`, `VideoFileDeleter`, `Task::DownloadVideo`/`DeleteVideoFile`,
and `DomainEvent::VideoAdded`/`VideoDeleted` are all hard-typed to
`playlist_id`. `Channel` (`channel-crud`) has no video relationship and no
`path` at all today.

Two alternative shapes were considered and rejected before this one:
- Giving every `Channel` a hidden backing `Playlist` row, reusing the
  playlist-reconcile machinery unchanged. Rejected: makes `Channel` silently
  masquerade as a kind of `Playlist`, which is a bigger modeling lie than the
  refactor it avoids.
- Keeping one `Video` entity but adding an owner-union (`playlist_id` or
  `channel_id`) directly on it, plus owner variants on `Task`/`DomainEvent`.
  Rejected: keeps `Video` coupled to knowing containers exist at all, and
  forces every consumer of `Video` (including already-shipped playlist code)
  to branch on owner kind.

## Goals / Non-Goals

**Goals:**
- Let `Playlist` and `Channel` each own videos without either knowing the
  other exists.
- Reuse the existing task/event dispatch machinery (`domain-events`,
  `task-scheduling`) without introducing owner-specific task types.
- Preserve today's behavior that the same YouTube video tracked by two
  containers gets two independent downloads (own status/quality/filename),
  not a shared/deduplicated one.
- Add channel video discovery capped at `Channel.video_limit`, sourced from
  `yt-dlp`, robust to titles containing arbitrary characters.

**Non-Goals:**
- Deduplicating or sharing a single downloaded file across containers.
- Any data migration path from the current schema — existing local
  databases are dropped and recreated (see proposal.md).
- Editing a channel's `path`/`quality`/`video_limit` after creation
  (channels stay creation-immutable, same as today).
- YouTube Data API-based channel video discovery — considered and rejected
  in favor of `yt-dlp` (see Decisions).

## Decisions

### Video / PlaylistVideo / ChannelVideo split
`Video` becomes container-agnostic: its own surrogate id, `youtube_id`,
`title`, `status`, `quality`, `filename`, `created_at`, `updated_at`. It has
no `playlist_id`/`channel_id` and no knowledge that containers exist.

`PlaylistVideo` (own id, `playlist_id`, `video_id -> Video.id`, `position`,
`created_at`) and `ChannelVideo` (own id, `channel_id`, `video_id`,
`position`, `created_at`) are the only things that know a `Video` belongs to
a container. Each gets its own repository (`PlaylistVideoRepository`,
`ChannelVideoRepository`) alongside a rewritten `VideoRepository` — no
repository performs a cross-entity join; a domain service orchestrates
creating/updating/deleting across them explicitly (e.g. on discovering a new
video: create the `Video` row, then create the owning `PlaylistVideo`/
`ChannelVideo` row, then publish the event).

A `Video` row is owned by exactly one relation row, never shared — the same
YouTube video tracked by two containers is two independent `Video` rows.
`position` lives on the relation, not `Video`: it's "where does this sit in
*this container's* listing" (YouTube-defined order for a playlist, recency
rank for a channel), not a fact about the file itself.

Cascades: deleting a `Playlist`/`Channel` deletes its relation rows; since a
relation row is a `Video` row's only owner, that cascades one level further
to delete the `Video` row too. This mirrors what `delete_all_for_playlist`
already does today, just walking one extra hop.

### Tasks stay single and container-agnostic
`Task::DownloadVideo { video_id, quality, output_dir }` and
`Task::DeleteVideoFile { filename, output_dir }` are unchanged in kind (no
`DownloadPlaylistVideo`/`DownloadChannelVideo` split) and drop the
`playlist_id`/lookup they have today. `VideoDownloader`/`VideoFileDeleter`
lose their `PlaylistRepository` dependency entirely — they only need
`VideoRepository` (to update status) and the primitives already on the task
payload. All container-specific resolution (which repository holds this
container's `quality`/`path`) happens once, upstream, in whichever
subscriber schedules the task.

### Per-container domain events, not an owner enum
`DomainEvent::VideoAdded`/`VideoDeleted` are replaced by
`VideoAddedToPlaylist`/`VideoRemovedFromPlaylist` and
`VideoAddedToChannel`/`VideoRemovedFromChannel`, each with its own
subscriber (mirroring `DownloadVideoOnVideoAdded`/
`DeleteVideoFileOnVideoDeleted`). Considered and rejected: a single event
type carrying an `Owner { Playlist(PlaylistId) | Channel(ChannelHandle) }`
enum with one subscriber branching on it — less total code, but these events
are already naturally published by two different domain services (playlist
reconcile/custom-playlist add-remove vs. channel reconcile), so keeping them
separate matches where they actually originate and keeps each subscriber a
trivial, single-purpose handler like every other one in this codebase.

### Channel video discovery via yt-dlp, not the YouTube Data API
Fetches `yt-dlp --flat-playlist --print-json -I 1:<video_limit>` against
`https://www.youtube.com/<handle>/videos`, parsing one JSON object per
output line (`id`, `title`) rather than the delimited
`"%(title)s | https://youtu.be/%(id)s"` format used during prototyping —
delimited text breaks if a title contains `|` (or a newline); JSON output
doesn't have that failure mode. Output order is newest-first, mapped
directly to `ChannelVideo.position` (0 = most recent).

Rejected alternative: the YouTube Data API's implicit "uploads" playlist
(derived by swapping a channel id's `UC` prefix for `UU`), which would have
let `list_current_videos` reuse `YoutubePlaylistItemsRepository` unchanged.
Rejected because that derivation is an undocumented internal convention, not
a stable public contract, and the app already hard-depends on `yt-dlp`
working correctly for every download, so leaning on it for listing too adds
no new class of failure.

### Channel top-N eviction reuses the existing diff, unchanged
`ChannelVideoReconciler`'s "current members" is exactly the top-N videos
`yt-dlp` returns. A previously-stored `ChannelVideo` absent from that set —
whether the underlying video was deleted/unlisted or has simply aged past
position N — is evicted by the same diff logic `sync_playlist_membership`
already uses for a YouTube-linked playlist: delete the relation (cascading
to its `Video`), publish `VideoRemovedFromChannel`. No separate "N+1 eviction"
concept needed.

### Channel gains `path`, required at creation
Same shape and validation as `Playlist.path` (reusing `PlaylistPath` or an
equivalent value object), since channel downloads need an output directory
exactly like playlist downloads do. `create_channel` becomes a 4-field call
(`channel`, `quality`, `video_limit`, `path`) instead of 3.

### One reconcile-interval config, shared
Channel recurring reconcile reuses the existing configurable reconcile
interval (default 3600s) rather than introducing a second knob — nothing
about the interval's purpose differs between a playlist and a channel.

### Schema collapse, no migrations
Every `CREATE TABLE` is rewritten to declare its final shape directly (the
one existing `ALTER TABLE ... ADD COLUMN position` is removed, and no new
`ALTER TABLE` is introduced anywhere). Since `Video`/`PlaylistVideo`/
`ChannelVideo` replace the current `videos` table's shape entirely, there is
no meaningful migration to write; the schema is simply re-created.

## Risks / Trade-offs

- Refactoring `Video` touches already-shipped, tested playlist code (not
  just net-new channel code) → mitigated by keeping the same repository/
  task/event *shapes* (CQS repositories, agnostic tasks) so most playlist
  tests get re-pointed at the new `PlaylistVideo`+`Video` split rather than
  rewritten from scratch.
- `yt-dlp` scraping the `/videos` tab is less stable than a typed API and
  can break if YouTube changes page structure → already an accepted risk
  for downloads today; mitigated the same way, via the existing
  `update-ytdlp` self-update task.
- A clean-but-empty or malformed `yt-dlp` listing result (e.g. a private/
  terminated channel) needs to fail the same safe way `download_video`
  already does (`Ok` with no videos / a clean non-zero exit) rather than
  hard-erroring the whole reconcile pass.
- The same YouTube video tracked by two containers still wastes disk/
  bandwidth downloading twice — explicitly out of scope (Non-Goals).

## Migration Plan

No data migration: this ships as a breaking schema change. Operators must
stop the daemon, delete the existing SQLite database file, and start the new
version, which recreates every table from its final `CREATE TABLE`
statement. Rollback is: stop the new version, restore the previous binary
and a pre-upgrade database backup (if one exists) — there is no forward/
backward schema compatibility to lean on otherwise.
