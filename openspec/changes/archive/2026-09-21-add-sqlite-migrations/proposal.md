## Why

Schema changes today mean dumping the whole SQLite database (and re-downloading every video), because schema creation is scattered across 7 repository constructors as idempotent `CREATE TABLE IF NOT EXISTS` calls with no version tracking and no `ALTER TABLE` path. Yarrtube is now running in production, so a schema change needs a way to evolve the existing database in place.

## What Changes

- Add `rusqlite_migration` and a `migrations/` directory of versioned SQL files, applied automatically once at `serve` startup, before any repository opens its connection.
- `migrations/0001_baseline.sql` consolidates the 8 existing `CREATE TABLE IF NOT EXISTS` statements verbatim, so an already-deployed production database is recognized as already at version 1 (no data loss, no re-create).
- Remove schema creation from the 7 repository constructors (`SqliteChannelRepository`, `SqlitePlaylistRepository`, `SqlitePlaylistVideoRepository`, `SqliteVideoRepository`, `SqliteVideoMetadataRepository`, `SqliteChannelVideoRepository`, `SqliteTaskRepository`) and from `SqliteEventRepository` / `SqliteEventPublisher` — the migration runner is now the single source of truth for schema.
- Forward-only migrations (no down/rollback support) for now.
- Repository unit tests that hand-roll their own in-memory schema switch to applying the same migrations, so tests exercise the real production schema.

## Capabilities

No spec-level behavior changes — this is purely an internal persistence/startup mechanism. Downloading, tracking, and serving playlists/channels behave identically; only how the schema is created and evolved changes.

## Impact

- `Cargo.toml`: new `rusqlite_migration` dependency.
- `migrations/0001_baseline.sql`: new file.
- `src/infrastructure/shared/sqlite_migrations.rs`: new module wrapping `rusqlite_migration`.
- `src/serve.rs`: runs migrations once at startup, before `build_application()`.
- The 7 repository files plus `event_repository.rs` / `event_publisher.rs`: lose their `CREATE TABLE` calls (and, where that was their only fallible step, their constructors stop returning `anyhow::Result`).
- Repository test modules in `sqlite_playlist_video_repository.rs` and `sqlite_channel_video_repository.rs`: their in-memory test fixture switches from a hand-written partial schema to the real migrations.
