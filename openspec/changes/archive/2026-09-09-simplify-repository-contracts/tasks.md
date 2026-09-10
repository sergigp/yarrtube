## 1. `VideoRepository` contract

- [x] 1.1 Replace `delete_not_in(playlist_id, current_ids)` with `delete(playlist_id, video_id)` on the `VideoRepository` trait, `SqliteVideoRepository`, and `FakeVideoRepository`; verify with a unit test that deletes one named video and asserts only that one is gone.
- [x] 1.2 Replace `upsert(video)` with `save(video)` (plain insert-or-full-replace of every column, no conflict-merge policy) on the trait, `SqliteVideoRepository`, and `FakeVideoRepository`; verify with a unit test that `save` on an existing row overwrites every field, including `status`.
- [x] 1.3 Update `VideoService::sync_playlist_videos` to decide new-vs-existing itself: `find` each synced video, `save` a freshly created `Video` when absent (publishing `VideoAdded` as today), or `save` an updated value (existing `status`/`created_at`, refreshed `title`/`updated_at`) when present; verify existing tests for "new video persisted as PENDING" and "existing video's status left unchanged" still pass unmodified in intent.
- [x] 1.4 Update `VideoService::sync_playlist_videos`'s removal loop to call `video_repository.delete(&id, &stored.video_id)` per video it identifies as no longer present, replacing the single `delete_not_in` call; verify with a unit test that removing one video from a two-video playlist deletes only that one.
- [x] 1.5 Update `sqlite_video_repository.rs`'s existing test suite (`it_should_delete_videos_no_longer_in_the_current_set`, `it_should_delete_all_videos_when_the_current_set_is_empty`, `it_should_leave_the_status_untouched_when_upserting_an_existing_video`, etc.) to exercise `delete`/`save` instead of the removed methods, preserving the same assertions.

## 2. `PlaylistRepository` contract and event publishing

- [x] 2.1 Replace `insert_with_event`/`delete_with_event` with plain `insert(playlist)`/`delete(playlist_id)` on the `PlaylistRepository` trait, `SqlitePlaylistRepository`, and `FakePlaylistRepository`; drop the repository's `create_events_table` call and its `sqlite_event_repository` import.
- [x] 2.2 Simplify `SqlitePlaylistRepository::new` to accept a plain `Connection` (owning its own internal `Mutex<Connection>`), matching `SqliteVideoRepository::new`'s shape, since it no longer shares a connection/transaction with the event repository.
- [x] 2.3 Add `event_publisher: Arc<dyn EventPublisher>` to `PlaylistService`; update `create_playlist`/`delete_playlist` to call `self.repository.insert(&playlist)` / `.delete(&id)` followed by `self.event_publisher.publish(&event)` as separate calls; verify with unit tests (using `FakeEventPublisher`) that the right event is published on create and on delete.
- [x] 2.4 Update `sqlite_playlist_repository.rs`'s existing test suite (transactional-commit tests for insert/delete-with-event) to reflect that publishing is no longer the repository's job — move those event-published assertions into `PlaylistService`'s tests per 2.3, and keep only plain persistence assertions (row present/absent) at the repository level.

## 3. Wiring

- [x] 3.1 Update `serve.rs::build_application()`: give `SqlitePlaylistRepository` its own `open_connection()` result instead of sharing `playlist_events_conn` with `SqliteEventRepository`; pass `event_repository.clone() as Arc<dyn EventPublisher>` into `PlaylistService::new`.
- [x] 3.2 Verify `cargo build --release` succeeds and `cargo test --locked` passes with the updated wiring.

## 4. Documentation

- [x] 4.1 Update `CLAUDE.md`'s Architecture section to remove the "Playlists and events share one SQLite connection... don't split that connection apart without preserving that atomicity" note, since it no longer reflects how the repositories are wired.

## 5. Final verification

- [x] 5.1 Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, and `cargo test --locked`; confirm all three are clean before considering this change complete.
