# Tasks — plex-collections-sync

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md across all layers, wired end-to-end: `domain/plex/` (`PlexItem`, `PlexCollection`), `PlexCollectionRepository` port + `HttpPlexCollectionRepository` + `FakePlexCollectionRepository`, `PlexCollectionReconciler` + `PlexCollectionDeleter` domain services, `ReconcilePlexCollectionsTask` + `schedule_reconcile_plex_collections_if_absent`, both delete subscribers, `Task::ReconcilePlexCollections` variant (kind `reconcile_plex_collections`, empty payload, Light lane via default arm), `name` field added to `PlaylistDeleted`/`ChannelDeleted` events (deleters publish it), task routed in `task_executor.rs`, and `serve.rs` reading `YARRTUBE_PLEX_URL`/`YARRTUBE_PLEX_TOKEN`/`YARRTUBE_PLEX_SECTION_ID`/`YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` and wiring repository/services/task/subscribers/seed only when the first three are set. Bodies return trivial hardcoded values (`Ok(())`, empty `Vec`), never `todo!()`. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_create_a_collection_for_a_playlist_with_scanned_videos` — the reconcile pass creates a collection (alphabetical sort, seeded members) for a playlist whose downloaded videos Plex has scanned, and schedules the next task.
- [x] 2.2 `it_should_create_a_collection_for_a_channel_with_scanned_videos` — same convergence for a channel.
- [x] 2.3 `it_should_skip_creating_a_collection_if_no_video_is_scanned_yet` — no empty collection when Plex has scanned none of the playlist's downloaded videos.
- [x] 2.4 `it_should_ignore_videos_that_are_not_downloaded` — pending/errored videos never become collection members even when scanned.
- [x] 2.5 `it_should_add_newly_scanned_videos_to_an_existing_collection` — only the missing rating key is added to an existing collection.
- [x] 2.6 `it_should_remove_videos_no_longer_tracked_from_the_collection` — a member whose video left yarrtube's state is removed.
- [x] 2.7 `it_should_do_nothing_if_already_in_sync` — a pass over converged state performs no collection mutations (idempotence).
- [x] 2.8 `it_should_continue_reconciling_remaining_collections_if_one_fails` — one collection's failure is logged and skipped; the rest still reconcile and the task returns Ok.
- [x] 2.9 `it_should_reschedule_the_next_reconcile_if_the_pass_fails` — a pass-wide failure (`list_items`) is logged and swallowed (the handler returns Ok, like `update_ytdlp`, so queue retries never stack extra recurring chains) and the next task is still scheduled.
- [x] 2.10 `it_should_delete_the_collection_if_a_playlist_is_deleted` — the `playlist_deleted` subscriber deletes the collection matching the event's `name`.
- [x] 2.11 `it_should_skip_if_no_collection_matches_the_playlist_name` — missing collection is a no-op, not an error.
- [x] 2.12 `it_should_delete_the_collection_if_a_channel_is_deleted` — proves the channel subscriber's wiring.
- [x] 2.13 `it_should_publish_the_playlist_name_on_deletion` — the outbox `playlist_deleted` payload includes `name` (existing playlist-delete adapter tests).
- [x] 2.14 `it_should_publish_the_channel_name_on_deletion` — same for `channel_deleted`.

## 3. Infrastructure adapters (TDD)

`HttpPlexCollectionRepository` against mockito:

- [x] 3.1 `it_should_list_section_items_with_their_youtube_ids` — parses the section listing (JSON via `Accept: application/json`), keeping only items with a `youtube://` guid.
- [x] 3.2 `it_should_list_collections` — parses `/library/sections/<id>/collections` into `PlexCollection`s.
- [x] 3.3 `it_should_list_collection_items` — parses a collection's children into `PlexItem`s.
- [x] 3.4 `it_should_create_a_collection_with_alphabetical_sort` — fetches the machine id from `/identity` (cached), POSTs `/library/collections` with `sectionId`/`title`/`uri`, then sets the `collectionSort` pref.
- [x] 3.5 `it_should_add_items_to_a_collection` — PUT `/library/metadata/<key>/items` with the members `uri`.
- [x] 3.6 `it_should_remove_an_item_from_a_collection` — DELETE `/library/metadata/<key>/items/<ratingKey>`.
- [x] 3.7 `it_should_delete_a_collection` — DELETE `/library/collections/<key>`.
- [x] 3.8 `it_should_fail_if_the_server_replies_with_an_error` — a non-2xx response maps to `Err`.

## 4. Multiple libraries

Yarrtube-fed content can be split across several Plex libraries, so the
integration reconciles a list of sections (`YARRTUBE_PLEX_SECTION_ID`
accepts a comma-separated list).

- [x] 4.1 Rework the port and wiring for section-per-call: `list_items`/`list_collections`/`create_collection` take a `section_id`, `PlexConfig` loses its `section_id`, the reconciler and deleter take `section_ids: Vec<String>` and loop them (a failing section is logged, remaining sections still reconcile), `serve.rs` parses `YARRTUBE_PLEX_SECTION_ID` as a comma-separated list, and the fake keeps per-section state. Existing tests updated to a single configured section, all passing.
- [x] 4.2 `it_should_create_collections_in_their_own_sections` — two configured sections; a playlist's videos scanned in one, a channel's in the other; each collection is created in its own section only.
- [ ] 4.3 `it_should_delete_the_collection_from_every_configured_section` — the deleter removes the name-matching collection from both configured sections.
- [ ] 4.4 Update the docs (`doc/INSTALLATION.md`, `doc/PLEX.md`) for the comma-separated section list and the multi-library behavior.

## 5. Verification

- [x] 5.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass. (Re-run after section 4.)
- [ ] 5.2 Manual check against the real NAS Plex with the `YARRTUBE_PLEX_*` env vars set: a tracked playlist gets its collection created/updated in the Plex UI with alphabetical sort, and deleting a playlist removes its collection. Also verify a start *without* the Plex env vars schedules no `reconcile_plex_collections` task.
- [x] 5.3 Update `README.md` with the Plex integration setup (env vars, how to obtain a token, recommend the "hide items which are in collections" library setting) and verify the docs match the implemented env var names.
