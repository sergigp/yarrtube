# Tasks

Note: the user asked for no commits or pushes unless requested; skip the
per-task commit from the apply guidance for this change.

## 1. Walking skeleton

- [x] 1.1 Create `migrations/0030_lookup_indexes.sql` holding only a header comment, add `LOOKUP_INDEXES_SQL` and register `M::up(LOOKUP_INDEXES_SQL)` last in `migrations()`. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

None: no behaviour change.

## 3. Infrastructure adapters (TDD)

SQLite migrations (`sqlite_migrations.rs`):

- [x] 3.1 `it_should_create_the_lookup_indexes_on_a_fresh_database` — drives adding the two `CREATE INDEX IF NOT EXISTS` statements to 0030.
- [x] 3.2 `it_should_add_the_lookup_indexes_when_upgrading_a_database` — `migrate_to(29)` with existing rows, `apply`, indexes exist and rows are kept (likely a regression test that passes at once).

## 4. Verification

- [x] 4.1 Re-run `EXPLAIN QUERY PLAN` for every query in proposal.md on a freshly migrated database and confirm each "after" plan (`SEARCH … USING INDEX`) and that `find_pending` / `list_non_completed` still plan as `SCAN tasks` (no regression).
- [x] 4.2 `scripts/run-tests.sh --only rust` (fmt, clippy, tests) passes.
