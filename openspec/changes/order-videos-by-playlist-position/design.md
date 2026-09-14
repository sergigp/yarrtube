## Context

See proposal.md - Why. Relevant current state:

- `YoutubeApiPlaylistItemsRepository::fetch_page` (`src/infrastructure/repositories/youtube_playlist_items_repository.rs`) deserializes each `playlistItems` API item's `snippet` into `title` and `resourceId.videoId` only. YouTube also returns `snippet.position` (a zero-based integer, per page), which is unused today.
- `Video` (`src/domain/video/video.rs`) has no ordering field.
- `SqliteVideoRepository::list_for_playlist` (`src/infrastructure/repositories/sqlite_video_repository.rs`) hard-codes `ORDER BY video_id ASC`.
- `VideoService::sync_playlist_membership` (`src/domain/video/service.rs`) already loops over every video returned by `list_current_videos`, in API order, and for each one either creates a new `Video` or rebuilds an existing one via `Video { title: ..., updated_at: ..., ..stored }` before an unconditional `save`. Every stored video for a YouTube-linked playlist is touched on every pass, whether or not anything changed for it.

## Goals / Non-Goals

**Goals:**
- Make `GET /playlists/{id}/videos` return YouTube-linked playlist videos in the same order they appear on YouTube.
- Keep that order correct after a playlist is reordered on YouTube, without requiring a special "reorder detected" code path — reuse the existing full-resync-every-pass behavior.

**Non-Goals:**
- Custom playlist ordering (no position source exists; out of scope per proposal).
- Ordinal filename prefixes or any on-disk ordering.
- Exposing position itself in the HTTP response (only its ordering effect is required).

## Decisions

**Persist `snippet.position` verbatim rather than deriving order from array index.**
The API already returns an explicit, documented position per item, `0`-indexed within the whole playlist (not just the current page). Deserializing it directly is simpler and more robust than relying on `list_current_videos` preserving array order across pagination (which it happens to do today, but isn't a documented contract of `PlaylistVideo`). Add `pub position: i64` to `PlaylistVideo` and `#[serde(rename = "position")] position: i64` to `PlaylistItemSnippet`.

**Store position as a plain nullable `INTEGER` column on `videos`, NULL for Custom-playlist videos.**
Alternatives considered: a separate `video_positions` table keyed by `(playlist_id, video_id)`. Rejected — `position` is 1:1 with a video row, changes at the same cadence as the rest of the row, and is already written by the same `save`/`update` calls; a join table would add complexity with no benefit. NULL (rather than e.g. `-1` or `0`) represents "no known position" explicitly and is exactly what Custom-playlist videos have, keeping the `ORDER BY` fallback simple (see below).

**Update position for every video on every reconcile pass, not just newly-discovered ones.**
`sync_playlist_membership` already rebuilds every existing video's row (`title`, `updated_at`) on each pass and calls `save` unconditionally — adding `position` to that same rebuild is a small diff, not a new code path. This is what makes the "track live YouTube position" decision (confirmed with the user) essentially free: no extra YouTube calls, no extra writes beyond what already happens, no new "did position change" branch needed since the full row is already being rewritten.

**`ORDER BY position IS NULL, position ASC, video_id ASC`.**
Rows with a non-NULL position (YouTube-linked) sort first, by position; rows with NULL position (Custom) sort after, falling back to the existing `video_id ASC` order among themselves — preserving today's behavior for Custom playlists exactly, per the proposal's scope decision.

## Risks / Trade-offs

- **[Risk]** `snippet.position` is scoped to the position at the time of that API call; if the playlist is edited *during* a paginated fetch (items added/removed/reordered mid-fetch), returned positions across pages could be internally inconsistent for that one pass. → **Mitigation**: not new — `list_current_videos` is already not atomic across pages today (membership diffing has the same exposure), and the next reconcile pass (default: hourly) self-heals it, consistent with the reconciliation model's existing self-healing design.
- **[Risk]** Existing rows created before this change have no recorded position (NULL) until their next reconcile. → **Mitigation**: harmless — they sort after all positioned rows until the next reconcile pass (at most one interval) fills them in; no migration/backfill script needed given the short window and existing recurring reconcile.

## Migration Plan

- Additive `ALTER TABLE videos ADD COLUMN position INTEGER` (nullable, no default needed beyond NULL) in `SqliteVideoRepository::new`'s schema setup, alongside the existing `CREATE TABLE IF NOT EXISTS`. No backfill: positions populate naturally on each playlist's next reconcile pass.
- No rollback concerns beyond reverting the code — an unused nullable column left behind by a rollback is harmless.
