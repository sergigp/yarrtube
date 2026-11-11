# Tasks

## 1. Walking skeleton

- [x] 1.1 Add `show_locks: Arc<ShowLocks>` as the last `PlaylistDeleter::new` / `ChannelDeleter::new` parameter (stored, not used yet); pass `Arc::new(ShowLocks::default())` in every test construction and the container's shared `show_locks` in `src/serve.rs`. The `Reconciling` variants, the `lock` helper and the 409 mapping arrive with 2.1 / 2.3 (a never-constructed variant or uncalled helper would fail clippy `-D warnings`; a stub lock can't refuse without blocking). Done when `cargo build` succeeds and all existing tests pass

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_fail_to_delete_a_playlist_while_it_is_being_synced`: a held lock on the playlist makes the delete refuse with 409 and change nothing (adds `DeletePlaylistError::Reconciling`, `PlaylistDeleter::lock`, the 409 mapping)
- [x] 2.2 `it_should_delete_a_playlist_while_another_source_is_being_synced`: a lock on another playlist doesn't block the delete
- [x] 2.3 `it_should_fail_to_delete_a_channel_while_it_is_being_synced`: same as 2.1 for a channel (adds `DeleteChannelError::Reconciling`, `ChannelDeleter::lock`, the 409 mapping)
- [x] 2.4 `it_should_delete_a_channel_while_another_source_is_being_synced`: same as 2.2 for a channel

## 3. Infrastructure adapters (TDD)

None.

## 4. Verification

- [ ] 4.1 `e2e-tests/helpers/fixtures.js`: `deleteAllSources` retries a delete refused with 409 (poll until it succeeds, 30 s timeout), so cleanup doesn't flake while a sync runs
- [ ] 4.2 Run `scripts/run-tests.sh --only web --only rust`; fmt, clippy and tests pass
- [ ] 4.3 Run `scripts/run-tests.sh --only e2e` (the e2e cleanup helper changed); all pass
