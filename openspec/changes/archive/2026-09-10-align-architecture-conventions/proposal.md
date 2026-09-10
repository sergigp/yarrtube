## Why

The `rust-architect` skill was recently rewritten to be shorter and less bloated. An audit of the codebase against the new wording surfaced a handful of concrete mismatches — repository file/placement conventions, an unneeded staleness guard in the task/event poll loop, and domain-level tests that duplicate (or under-cover relative to) application-layer tests. This change closes those gaps and updates the skill wording where the codebase's existing behavior is what we actually want to keep.

## What Changes

- Rephrase the skill's `infrastructure/shared/` vs `infrastructure/repositories/` distinction: `shared/` holds infra usable across aggregates (e.g. domain events, the clock), not "infra with no trait/port boundary"; `repositories/` holds one aggregate's dedicated port implementation.
- Codify the existing `<implementation>_<port>.rs` repository file-naming convention explicitly in the skill (already the de facto style everywhere except one file).
- Rename `infrastructure/repositories/video_file_repository.rs` to `system_video_file_repository.rs` (and its implementation struct) to match that convention.
- Split `infrastructure/repositories/sqlite_event_repository.rs` into separate `EventPublisher` and `EventRepository` files, and relocate them — together with `system_clock.rs` — from `infrastructure/repositories/` into `infrastructure/shared/domain_events/` and `infrastructure/shared/` respectively.
- Simplify `TaskRepository::list_eligible` and `EventRepository::list_eligible` to return entities directly instead of IDs followed by a per-id `find`, and remove the now-dead "task/event disappeared before dispatch" branch this two-phase read exists to guard — there is currently only one poll-loop instance and no other writer that deletes/updates an already-eligible row, so the guard protects against nothing that can happen today.
- Delete the test modules in `domain/playlist/service.rs` and `domain/video/service.rs`, after first porting the `download_video`/`delete_video_file` scenarios not yet covered at the task-handler layer into `tasks/download_video_task.rs` and `tasks/delete_video_file_task.rs`, so no behavioral coverage is lost in the move.

**BREAKING**: none — this is an internal restructuring with no change to HTTP contract, CLI behavior, persisted schema, or runtime behavior.

## Capabilities

No capability specs are created or modified — this is a pure internal refactor (file layout, naming, test placement) with no change in observable behavior. `skip_specs: true` is set in `.openspec.yaml`.

## Impact

- `.claude/skills/rust-architect/SKILL.md` — reworded `shared/` vs `repositories/` guidance, codified file-naming rule.
- `src/infrastructure/repositories/` — `video_file_repository.rs` renamed; `sqlite_event_repository.rs` removed (split and relocated); `system_clock.rs` removed (relocated).
- `src/infrastructure/shared/` — new `domain_events/` module (`EventPublisher`, `EventRepository`); `system_clock.rs` added.
- Every importer of the moved/renamed items: `domain/playlist/service.rs`, `domain/video/service.rs`, `subscribers/*.rs`, `tasks/*.rs`, `serve.rs` composition root, and their test modules.
- `src/tasks/download_video_task.rs`, `src/tasks/delete_video_file_task.rs` — new test cases.
- `src/domain/playlist/service.rs`, `src/domain/video/service.rs` — test modules removed.
- No public API, CLI, or persisted-data changes.
