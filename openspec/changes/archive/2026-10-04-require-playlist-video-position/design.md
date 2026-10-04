## Files

- `migrations/0006_playlist_video_position_required.sql`: new. Rebuilds `playlist_videos` with `position INTEGER NOT NULL` (SQLite can't alter a column's nullability), copying every row.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers migration 0006 and gets its migration tests.
- `src/domain/playlist_video/playlist_video.rs`: `position: i64`. The position-less `create` is removed and `create_with_position` is renamed to `create`.
- `src/infrastructure/repositories/sqlite_playlist_video_repository.rs`: maps `position` as `i64`. `ORDER BY position IS NULL, ...` becomes `ORDER BY position, id`. The no-position tests are removed.
- `src/domain/services/playlist_video_reconciler.rs`: the snapshot's `positions` map is built without `?`, and the position comparison and save drop `Some(..)`.
- `src/domain/services/video_downloader.rs`: `playlist_video.map(|pv| pv.position)`.
- Test seeds across `application/` that call `PlaylistVideo::create(..)` without a position now pass one.

## Types & Signatures

```rust
pub struct PlaylistVideo {
    pub id: i64,
    pub playlist_id: PlaylistId,
    pub video_id: VideoRecordId,
    pub position: i64,
    pub created_at: DateTime<Utc>,
}

impl PlaylistVideo {
    pub fn create(playlist_id: PlaylistId, video_id: VideoRecordId, position: i64, now: DateTime<Utc>) -> Self;
}
```

```sql
-- 0006: CREATE playlist_videos_new (same columns, position INTEGER NOT NULL, UNIQUE (playlist_id, video_id));
-- INSERT INTO playlist_videos_new SELECT * FROM playlist_videos; DROP TABLE playlist_videos; ALTER TABLE playlist_videos_new RENAME TO playlist_videos;
```

A `NULL` position deliberately fails the migration instead of being dropped or defaulted. Prod was checked to have none, so a failure would mean unexpected data.

## Call Stack

- Startup: `sqlite_migrations::apply(conn)` → `Migrations::to_latest` runs 0006 once.
- Reconcile: `take_local_snapshot` → `positions: HashMap<VideoRecordId, i64>` from every playlist video → `generate_metadata(video, dir, positions.get(&video.id).copied())`. The value stays an `Option`, because `resolve_sorttitle` is shared with channels.
- Download: `VideoDownloader` → `playlist_video_repository.find_by_video(&id)` → `Option<PlaylistVideo>` → `.map(|pv| pv.position)` → `resolve_sorttitle`.

## Test Plan

1. Behaviour tests
   - None new. The change is type-level; the existing reconcile, download and listing acceptance tests stay green with position-bearing seeds.
2. Infrastructure tests
   - `sqlite_migrations::it_should_keep_playlist_videos_when_making_position_required`: rows inserted at version 5 survive `apply` unchanged.
   - `sqlite_migrations::it_should_reject_a_playlist_video_without_position`: after `apply`, inserting a `NULL` position fails.
   - `sqlite_playlist_video_repository`: the existing round-trip and ordering tests are updated; the no-position tests are deleted.
