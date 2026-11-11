# Design

Pure schema change: one migration, no Rust types or call paths change. See
proposal.md for the indexes, the queries they serve and the plans.

## Files

- `migrations/0030_lookup_indexes.sql` — new; the two `CREATE INDEX IF NOT
  EXISTS` statements, one comment each naming the queries served.
- `src/infrastructure/shared/sqlite_migrations.rs` — register the migration
  (`LOOKUP_INDEXES_SQL` const + `M::up` at the end of `migrations()`); its tests.

## Types & Signatures

```rust
const LOOKUP_INDEXES_SQL: &str = include_str!("../../../migrations/0030_lookup_indexes.sql");
// migrations(): ..., M::up(EVENT_RUN_AT_SQL), M::up(LOOKUP_INDEXES_SQL)
```

```sql
CREATE INDEX IF NOT EXISTS idx_channel_videos_video ON channel_videos (video_id);
CREATE INDEX IF NOT EXISTS idx_playlist_videos_video ON playlist_videos (video_id);
```

No `tasks` expression index: it would make writes of a row with a
malformed JSON payload fail (see proposal.md, Considered and left out).

## Call Stack

`serve` / CLI startup → `sqlite_migrations::apply(conn)` →
`Migrations::new(migrations()).to_latest(conn)` → runs 0030 once (tracked by
`user_version`). Repositories unchanged; SQLite's planner picks the indexes.

## Test Plan

No behaviour tests (no behaviour change).

Infrastructure — `sqlite_migrations.rs` (real in-memory SQLite):

1. `it_should_create_the_lookup_indexes_on_a_fresh_database` — after
   `apply`, `sqlite_master` lists `idx_channel_videos_video`,
   `idx_playlist_videos_video` on their tables.
2. `it_should_add_the_lookup_indexes_when_upgrading_a_database` —
   `migrate_to(29)`, insert a channel video and a playlist video,
   `apply`; both indexes exist and the rows are kept.

Idempotence is already covered by `it_should_be_a_no_op_when_applied_twice`.
