# Proposal

## Why

Several hot lookups scan whole tables. `channel_videos` and `playlist_videos`
are only indexed by `UNIQUE(channel_id, video_id)` / `UNIQUE(playlist_id,
video_id)`, which cannot serve a lookup by `video_id` alone, yet
`find_by_video` runs per video in loops (`source_of` in
`video_searcher.rs` on every Home poll, `VideoOwnerFinder`, the category
collector, the task view searcher).

## What Changes

One new migration, `0030_lookup_indexes.sql`, idempotent (`CREATE INDEX IF
NOT EXISTS`), adding two indexes. No query rewrites, no repository API
changes, no behaviour change.

Plans below are `EXPLAIN QUERY PLAN` on a database migrated to 0029, before →
after. Timings: mean of 200 runs via Python `sqlite3` on a seeded copy
(20 000 `channel_videos`, 5 000 `playlist_videos`, 6 000 `tasks` —
three task types, 12 running).

### `idx_channel_videos_video ON channel_videos (video_id)`

| Query (file) | Before | After |
|---|---|---|
| `find_by_video`: `SELECT … FROM channel_videos WHERE video_id = ?1` (`sqlite_channel_video_repository.rs`) | `SCAN channel_videos` | `SEARCH channel_videos USING INDEX idx_channel_videos_video (video_id=?)` |
| `DELETE FROM channel_videos WHERE video_id = ?1` (same) | `SCAN channel_videos` | `SEARCH … USING INDEX idx_channel_videos_video (video_id=?)` |
| `is_channel_video`: `SELECT EXISTS(SELECT 1 FROM channel_videos WHERE video_id = ?1)` (`sqlite_video_repository.rs`) | `SCAN channel_videos USING COVERING INDEX sqlite_autoindex_channel_videos_1` | `SEARCH … USING COVERING INDEX idx_channel_videos_video (video_id=?)` |
| `last_sibling_episode_of_day`, inner `SELECT channel_id FROM channel_videos WHERE video_id = ?1` (same) | `SCAN channel_videos USING COVERING INDEX sqlite_autoindex_channel_videos_1` | `SEARCH … USING INDEX idx_channel_videos_video (video_id=?)` |

`find_by_video`: 393 µs → 5 µs.

### `idx_playlist_videos_video ON playlist_videos (video_id)`

| Query (file) | Before | After |
|---|---|---|
| `find_by_video`: `SELECT … FROM playlist_videos WHERE video_id = ?1` (`sqlite_playlist_video_repository.rs`) | `SCAN playlist_videos` | `SEARCH playlist_videos USING INDEX idx_playlist_videos_video (video_id=?)` |
| `DELETE FROM playlist_videos WHERE video_id = ?1` (same) | `SCAN playlist_videos` | `SEARCH … USING INDEX idx_playlist_videos_video (video_id=?)` |

`find_by_video`: 99 µs → 4 µs.

### Considered and left out

- **`tasks (json_extract(payload, '$.video_id'), task_type)`** — an
  expression index matching the `schedule` dedupe,
  `find_non_completed_for_video` and `delete_pending_for_video` text exactly
  (all `SCAN tasks` → `SEARCH tasks USING INDEX idx_tasks_video (<expr>=? AND
  task_type=?)`; `find_non_completed_for_video` 180 → 5 µs). Dropped during
  apply: SQLite evaluates the expression on every write, so inserting a row
  whose payload is not valid JSON fails with `malformed JSON`
  (`it_should_read_a_row_it_cannot_decode_as_unreadable` broke), and building
  the index over an existing such row would fail migration 0030 and stop the
  daemon from starting. A safe expression (e.g. guarded by `json_valid`)
  needs the queries rewritten to match — see follow-ups.

- **`tasks (status, run_at)`** — the suggested eligible-rows index. It
  regresses other queries: nearly every row is `pending`, so `status` barely
  filters, and with no `sqlite_stat1` the planner still picks it.
  `find_pending` 77 → 831 µs (`SCAN tasks` → `SEARCH … (status=?)` + `USE
  TEMP B-TREE FOR ORDER BY`), `list_non_completed` 4.0 → 5.5 ms.
- **Partial `tasks (run_at) WHERE status = 'pending'`** — no regressions;
  `list_eligible` goes `SCAN tasks` + `USE TEMP B-TREE FOR ORDER BY` →
  `SEARCH tasks USING INDEX idx_tasks_pending_run_at (run_at<?)`, but saves
  only 0.2–0.4 ms per scheduling pass at 6 000 tasks (552 → 321 µs with 428
  due; 5.1 → 4.7 ms with all due). The cost is reading every eligible row,
  which an index does not remove. Left out as not measurable; easy to add
  later if wanted.
- **`list_running` / `list_non_completed` / `find_pending`** — stay `SCAN
  tasks`; none has a selective predicate an index could use without the
  regressions above.
- **`events`** — no index. Rows are deleted as soon as they are dispatched
  (`domain_events_consumer.rs`) or dead-lettered, so the table holds only
  in-flight events (a handful, briefly). `list_eligible` / `list_pending`
  stay `SCAN events`; update/delete are by primary key.
- `tasks` lookups by `id` (`claim`, `update`, `delete`) already use the
  primary key.

### Follow-ups (need query rewrites, out of scope)

- Per-video task lookups: rewrite the three `json_extract(payload,
  '$.video_id')` filters to an expression that cannot raise (e.g. `CASE WHEN
  json_valid(payload) THEN json_extract(payload, '$.video_id') END`) or move
  `video_id` into its own column, then index it. Biggest remaining win.

- `list_eligible` (tasks) returns every eligible row each pass though the
  executor only spawns what fits free lane slots; a per-lane `LIMIT` would
  cut the real cost, and then the partial `run_at` index would pay.
- `channel_videos … WHERE channel_id = ?1 ORDER BY position` uses a temp
  B-tree for the sort; a `(channel_id, position)` index would remove it.
  Not requested; listed only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — indexes change no observable behaviour; `skip_specs: true`.

## Impact

- `migrations/0030_lookup_indexes.sql` (new),
  `src/infrastructure/shared/sqlite_migrations.rs` (register it, tests).
- Upgrade cost: two index builds on first start; at NAS scale
  (tens of thousands of rows) well under a second.
- Slightly slower inserts into the two tables (one more B-tree each);
  negligible against yt-dlp/Ollama work.
