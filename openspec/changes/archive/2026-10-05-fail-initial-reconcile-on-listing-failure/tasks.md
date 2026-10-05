Note: the code for this change already exists, uncommitted, in the working tree (written during the PR audit). Each task confirms the existing code matches design.md and its test goes red without the behaviour. Nothing is rewritten.

## 1. Walking skeleton

- [x] 1.1 Confirm `initial_reconcile` is on `PlaylistVideoReconcilerApi` and `ChannelVideoReconcilerApi`, `ListingFailure` is threaded through `reconcile_*` → `sync_membership_with_youtube` → `list_current_videos` as in design.md, and both `*Created` subscribers call `initial_reconcile`. Verify: `cargo build` succeeds and `cargo test --locked` passes.

## 2. Behaviour (TDD)

- [x] 2.1 `reconcile_on_playlist_created::it_should_fail_if_listing_fails`: a failed listing on creation fails the event's processing, stores nothing, publishes nothing and schedules no recurring reconcile. Verify red by temporarily pointing the subscriber back at `reconcile` (the test fails on the result assert), then green by restoring `initial_reconcile`.
- [x] 2.2 `reconcile_on_channel_created::it_should_fail_if_listing_fails`: the same for channels. Verify red and green the same way.

## 3. Infrastructure adapters (TDD)

- [x] 3.1 None: no adapter changes.

## 4. Verification

- [x] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass, and `scripts/run-smoke-tests.sh` passes (creation flows are user-visible).
- [x] 4.2 `openspec validate fail-initial-reconcile-on-listing-failure --strict` passes.
