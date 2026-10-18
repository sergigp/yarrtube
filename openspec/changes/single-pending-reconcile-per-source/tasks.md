## 1. Walking skeleton

- [x] 1.1 Create every signature from design.md: `ChannelDeleter::new` and `PlaylistDeleter::new` take `task_repository`, private `remove_pending_reconciles` returning `Ok(())` and called from `delete`; `serve.rs` and every test constructing a deleter pass a `SqliteTaskRepository`. Reconcilers unchanged. Done when `cargo build`, `cargo test --locked` pass with no new tests.

## 2. Behaviour (TDD)

Channel HTTP:
- [x] 2.1 `it_should_remove_the_pending_reconcile_of_a_deleted_channel`: pending reconcile of the channel and of another channel → only the other's remains.
- [x] 2.2 `it_should_remove_every_pending_reconcile_of_a_deleted_channel`: two pending reconciles of the channel → none remain.

Playlist HTTP:
- [x] 2.3 `it_should_remove_the_pending_reconcile_of_a_deleted_playlist`: pending reconcile of the playlist and of another playlist → only the other's remains.
- [x] 2.4 `it_should_remove_every_pending_reconcile_of_a_deleted_playlist`: two pending → none remain.

Reconcile tasks:
- [x] 2.5 `reconcile_channel_task::it_should_not_schedule_the_next_reconcile_if_one_is_already_pending`: pending reconcile at `now + 5 min` → task list is exactly that one after the pass.
- [x] 2.6 `reconcile_playlist_task::it_should_not_schedule_the_next_reconcile_if_one_is_already_pending`: same for a playlist.

## 3. Infrastructure adapters (TDD)

_None: no adapter changes._

## 4. Verification

- [x] 4.1 Tighten `smoke-tests/helpers/tasks.js` to expect exactly one pending sync per source label (the earlier specs delete and re-add the smoke playlist, so this exercises the re-add path), and drop its comment about leftover syncs.
- [x] 4.2 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `npm run check` all pass.
- [x] 4.3 Run `scripts/run-smoke-tests.sh`: all pass.
