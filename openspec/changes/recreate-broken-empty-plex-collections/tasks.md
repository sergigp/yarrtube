## 1. Behaviour (TDD)

- [x] 1.1 `reconcile_plex_collections_task::it_should_recreate_an_empty_collection_with_its_scanned_videos`: an existing empty collection with scanned desired videos is deleted and recreated with them (add `recreate_collection`, thread `section_id` into `converge_members`, branch on empty members + non-empty desired, log `recreated empty Plex collection`). Verify with `cargo test it_should_recreate_an_empty_collection_with_its_scanned_videos`.
- [ ] 1.2 `reconcile_plex_collections_task::it_should_leave_an_empty_collection_alone_if_no_video_is_scanned`: an existing empty collection with no scanned desired videos gets no mutations. Verify with `cargo test it_should_leave_an_empty_collection_alone_if_no_video_is_scanned`.

## 2. Infrastructure adapters (TDD)

`plex_collection_repository.rs`:

- [ ] 2.1 `it_should_fail_with_the_plex_response_body_if_the_server_replies_with_an_error`: `ensure_success` takes the response by value, reads the trimmed body into the error (`…: <body>`, suffix omitted when empty), and returns the response on success; update every caller. Existing `it_should_fail_if_the_server_replies_with_an_error` stays green unchanged. Verify with `cargo test plex`.

## 3. Verification

- [ ] 3.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 3.2 After deploying, logs show `recreated empty Plex collection` for any collection still empty (PBS Space Time / Control de Misión if not already deleted by hand) and no `failed to reconcile Plex collection` 400s on the next pass.
