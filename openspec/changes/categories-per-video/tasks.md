# Tasks

Requires `video-reconciler` to be implemented first (`VideoReconciler`, `SyncVideoTask`).

## 1. Walking skeleton

- [x] 1.1 Create every file, type and signature from design.md: migration `0014_video_categories.sql` (table + index + backfill) registered in `sqlite_migrations.rs`; `domain/video_category` (`VideoCategories`, `VideoCategoriesChanged`); `DomainEvent::VideoCategoriesChanged`; `VideoCategoriesRepository` + `SqliteVideoCategoriesRepository` (stub bodies: `find_many` → every id as `none`, `list_in_category` → empty, writes `Ok(())`) built in the infrastructure container; `VideoCategoryAssigner` + `VideoCategoryAssignerApi` (bodies `Ok(())`); new deps on `CategoryUpdater`, `CategoryDeleter`, `CategoryVideoCollector`, `VideoReconciler`, `ChannelVideoReconciler`, `PlaylistVideoReconciler`, `ChannelDeleter`, `PlaylistDeleter`, all wired in `src/serve.rs` and test constructors. `CategoryVideoCollector` keeps its member-walking body for now. Done when `cargo build` succeeds and all existing tests pass.

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_categorize_the_videos_of_an_added_member` — set members writes the member's records' sets and publishes one batched `VideoCategoriesChanged`.
- [x] 2.2 `it_should_uncategorize_the_videos_of_a_removed_member` — a source left in no category leaves its records in none.
- [x] 2.3 `it_should_keep_the_other_categories_of_a_member_removed_from_one` — sets are the source's remaining categories.
- [x] 2.4 `it_should_not_categorize_other_copies_of_a_member_video` — categories are per record, not per YouTube video.
- [x] 2.5 `it_should_uncategorize_stale_videos_of_a_former_member` — records still holding the category are recomputed (retry safety).
- [x] 2.6 `it_should_not_announce_a_members_set_that_changes_no_video` — only changed records are saved/announced.
- [x] 2.7 `it_should_uncategorize_the_videos_of_a_deleted_category` — deleter reassigns with the category excluded before deleting.
- [x] 2.8 `it_should_announce_the_videos_of_a_renamed_category` — rename publishes, rows untouched.
- [x] 2.9 `it_should_not_announce_a_recolored_category` — color-only update publishes nothing.
- [x] 2.10 Refactor `CategoryVideoCollector` to read `VideoCategoriesRepository::list_in_category` (record → source via `ChannelVideoRepository::list` / `PlaylistVideoRepository::list`, channel sources first, then member order). Extend the seeds of the existing category list/count/total, `list_for_category` and `mark_category_watched` tests with the matching `VideoCategories`; done when those suites pass unchanged in their assertions.
- [x] 2.11 `it_should_categorize_a_new_video_of_a_categorized_channel` — `VideoReconciler::reconcile` assigns the record from its owner.
- [x] 2.12 `it_should_leave_a_new_video_of_an_uncategorized_playlist_uncategorized` — AI regime untouched, no event.
- [ ] 2.13 `it_should_repair_the_categories_of_stored_videos` — channel reconcile pass calls `assign_source`.
- [ ] 2.14 `it_should_leave_the_categories_of_an_uncategorized_channel` — pass never clears AI-regime rows.
- [ ] 2.15 `it_should_delete_the_categories_of_a_removed_video` — reconciler removing a record deletes its rows.
- [ ] 2.16 `it_should_repair_the_categories_of_stored_playlist_videos` — playlist reconcile pass calls `assign_source`.
- [ ] 2.17 `it_should_delete_the_categories_of_a_deleted_channel_videos` — channel deletion deletes its records' rows.
- [ ] 2.18 `it_should_delete_the_categories_of_a_deleted_playlist_videos` — playlist deletion deletes its records' rows.

## 3. Infrastructure adapters (TDD)

SqliteVideoCategoriesRepository:
- [x] 3.1 `it_should_find_none_for_a_record_without_rows`
- [x] 3.2 `it_should_replace_a_record_set_on_save`
- [x] 3.3 `it_should_delete_all_rows_when_saving_an_empty_set`
- [x] 3.4 `it_should_list_records_in_a_category_with_their_full_sets`
- [x] 3.5 `it_should_delete_a_record_rows`

Migrations:
- [x] 3.6 `it_should_backfill_video_categories_from_members` — channel and playlist members, an uncategorized source, a record in two categories.

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings` all pass.
- [ ] 4.2 End-to-end suite passes via `scripts/run-tests.sh` (category list, feed, counts and mark watched unchanged).
- [ ] 4.3 Manual (agent): on a copy of a pre-upgrade database (not the user's `yarrtube.sqlite3`; copy it to a temp path first), start `scripts/run-local.sh --port <free port>` against the copy, and confirm via `GET /api/categories` and `GET /api/categories/<id>/videos` that counts and feeds equal those of the same copy on `main`; then set a category's members via the API and confirm `sqlite3` shows the expected `video_categories` rows and an `events` row of type `video_categories_changed`. Stop the server and delete the copy.
