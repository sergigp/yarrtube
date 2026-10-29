# Design

## Files

- `migrations/0014_video_categories.sql` — `video_categories(video_id, category_id)` table, PK `(video_id, category_id)`, index on `category_id`, backfill from `category_members` × `channel_videos` / `playlist_videos`.
- `src/infrastructure/shared/sqlite_migrations.rs` — registers 0014.
- `src/domain/video_category/mod.rs` — new aggregate module: a record's categories.
- `src/domain/video_category/video_categories.rs` — `VideoCategories` entity (one record's category set).
- `src/domain/video_category/events.rs` — `VideoCategoriesChanged` event.
- `src/domain/mod.rs` — declares `video_category`.
- `src/domain/event/domain_event.rs` — `DomainEvent::VideoCategoriesChanged` + type/payload arms.
- `src/infrastructure/repositories/sqlite_video_categories_repository.rs` — `VideoCategoriesRepository` port + SQLite adapter.
- `src/infrastructure/repositories/mod.rs`, `src/infrastructure/infrastructure_container.rs` — expose/build the repository.
- `src/domain/services/video_category_assigner.rs` — new `VideoCategoryAssigner`: computes source-regime sets, saves only changed ones, publishes the event.
- `src/domain/services/mod.rs` — exports it.
- `src/domain/services/category_updater.rs` — `set_members` reassigns after saving; `update` announces a rename.
- `src/domain/services/category_deleter.rs` — reassigns (category excluded) before deleting.
- `src/domain/services/category_video_collector.rs` — reads records from `video_categories` instead of walking members; same output and order.
- `src/domain/services/video_reconciler.rs` (from `video-reconciler`) — `reconcile` assigns the record's categories before planning its steps.
- `src/domain/services/channel_video_reconciler.rs`, `playlist_video_reconciler.rs` — assign the source after each pass (best effort); delete a removed record's categories with it.
- `src/domain/services/channel_deleter.rs`, `playlist_deleter.rs` — delete each record's categories with the record.
- `src/serve.rs` — builds `VideoCategoryAssigner`, injects it and the repository.
- `src/infrastructure/shared/category_lock.rs` (+ `shared/mod.rs`, a `category_lock` field in `infrastructure_container.rs`) — `CategoryLock`, the one in-process lock serializing video category writes.
- `src/domain/category/category_id.rs` — `CategoryId` derives `PartialOrd, Ord` (`VideoCategories::new` sorts).
- Test-only wiring of the new constructor deps: `domain/services/internal_video_reconciler.rs`, `application/subscribers/reconcile_on_{channel,playlist}_created.rs`, `application/tasks/{sync_video_task,reconcile_channel_task,reconcile_playlist_task}.rs`.
- Existing test modules seeding categorized videos (`application/http/{categories,videos,playlists,channels}/mod.rs`) — also seed `VideoCategories`.

## Types & Signatures

```rust
// domain/video_category/video_categories.rs
/// The categories one stored video record belongs to. Empty = in none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoCategories {
    pub video_id: VideoRecordId,
    /// Sorted, each once.
    pub category_ids: Vec<CategoryId>,
}
impl VideoCategories {
    pub fn new(video_id: VideoRecordId, category_ids: Vec<CategoryId>) -> Self; // sorts + dedups
    pub fn none(video_id: VideoRecordId) -> Self;
    /// The same record, no longer in `category_id`.
    pub fn without(self, category_id: &CategoryId) -> Self;
}

// domain/video_category/events.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VideoCategoriesChanged {
    /// Records whose category set, or a name in it, changed. Never empty.
    pub video_ids: Vec<String>,
}
impl VideoCategoriesChanged { pub const EVENT_TYPE: &str = "video_categories_changed"; }

// domain/event/domain_event.rs
pub enum DomainEvent { /* … */ VideoCategoriesChanged(VideoCategoriesChanged) }

// infrastructure/repositories/sqlite_video_categories_repository.rs
pub trait VideoCategoriesRepository: Send + Sync {
    /// One per requested id, `VideoCategories::none` when it has no row.
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategories>>;
    /// Every record belonging to `category_id`, with its full set.
    fn list_in_category(&self, category_id: &CategoryId) -> anyhow::Result<Vec<VideoCategories>>;
    /// Replaces each record's rows with its set, all in one transaction.
    fn save_many(&self, categories: &[VideoCategories]) -> anyhow::Result<()>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}
pub struct SqliteVideoCategoriesRepository { db: Database }

// domain/services/video_category_assigner.rs
pub struct VideoCategoryAssigner {
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    category_lock: Arc<CategoryLock>, // last `new` param
}
pub trait VideoCategoryAssignerApi: Send + Sync {
    /// Serializes video category writes. `assign_source` and `assign_video`
    /// take it themselves; callers of `reassign_category` hold it from before
    /// they change the categories it is computed from until it returns.
    fn lock(&self) -> MutexGuard<'_, ()>;
    /// Source regime for every record of `source`: its set = the source's
    /// categories. No-op when the source is in no category (AI regime).
    fn assign_source(&self, source: &CategoryMember) -> anyhow::Result<()>;
    /// Same, for one record of `source`.
    fn assign_video(&self, video_id: &VideoRecordId, source: &CategoryMember) -> anyhow::Result<()>;
    /// After `category_id`'s members changed or it was deleted: recomputes every
    /// record of `previous_members`, of its current members and of records
    /// still holding it, against `categories` (the state to apply). The
    /// records of a source in no category only lose `category_id`.
    fn reassign_category(
        &self,
        category_id: &CategoryId,
        previous_members: &[CategoryMember],
        categories: &[Category],
    ) -> anyhow::Result<()>;
    /// Announces a rename: one event naming every record holding `category_id`.
    fn announce_renamed(&self, category_id: &CategoryId) -> anyhow::Result<()>;
}

// domain/services/category_updater.rs — new deps
pub fn new(
    repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_category_assigner: VideoCategoryAssigner,
) -> Self;

// domain/services/category_deleter.rs — new dep
pub fn new(repository: Arc<dyn CategoryRepository>, video_category_assigner: VideoCategoryAssigner) -> Self;

// domain/services/category_video_collector.rs — new dep, same API
pub fn new(
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
) -> Self;

// infrastructure/shared/category_lock.rs
#[derive(Default)]
pub struct CategoryLock { held: Mutex<()> }
impl CategoryLock { pub fn lock(&self) -> MutexGuard<'_, ()>; } // poison ignored

// domain/services/video_reconciler.rs — new dep `video_category_assigner: VideoCategoryAssigner`
// ChannelVideoReconciler / PlaylistVideoReconciler / ChannelDeleter / PlaylistDeleter —
// new deps `video_category_assigner` (reconcilers) and `video_categories_repository` (all four).
// New constructor params are appended last; `ChannelDeleter::new` takes
// `#[allow(clippy::too_many_arguments)]` like the reconcilers.
```

Order matters on failure: `set_members` saves members then reassigns (a retry re-finds stale records via `list_in_category`); `delete` reassigns with the category excluded then deletes (a failed delete is healed by the members' next reconcile pass).

## Call Stack

**Set members** — `set_category_members` → `CategoryUpdater::set_members(&id, members)`
1. `VideoCategoryAssigner::lock()` (held to the end) → `find_category` (old) → `ensure_tracked` → `CategoryRepository::update(&new)`
2. `VideoCategoryAssigner::reassign_category(&id, &old.members, &CategoryRepository::list()?)`
   - sources = `old.members` ∪ members of `id` in `categories` ∪ sources of `list_in_category(&id)` (via `ChannelVideoRepository::list` / `PlaylistVideoRepository::list`)
   - per source: `source_category_ids(source, categories)`; its record ids via `list_for_channel` / `list_for_playlist` ⇒ desired `VideoCategories::new(id, ids)`; a source in no category (AI regime) ⇒ `find_many(ids)` → each `without(&category_id)`
   - `VideoCategoriesRepository::find_many(ids)` ⇒ keep desired ≠ current ⇒ `save_many(changed)`
   - changed non-empty ⇒ `EventPublisher::publish(VideoCategoriesChanged { video_ids })`

**Rename** — `CategoryUpdater::update(&id, CategoryUpdate { name: Some(_), .. })`
1. save as today → `VideoCategoryAssigner::announce_renamed(&id)` → `list_in_category(&id)` → publish if non-empty. Color-only update publishes nothing.

**Delete** — `CategoryDeleter::delete(&id)`
1. `lock()` (held to the end) → `find_category` → `reassign_category(&id, &category.members, &list()? without id)` → `CategoryRepository::delete(&id)`

**Video sync** — `SyncVideoTask` → `VideoReconciler::reconcile(&video_id)`
1. find video + owner (private `find_owner`, was `find_target`) ⇒ `(source: CategoryMember, target)`
2. `VideoCategoryAssigner::assign_video(&video.id, &source)` → `lock()` → `CategoryRepository::list()` → source ids empty ⇒ return; else `find_many([id])` → differs ⇒ `save_many` + publish
3. `reconcile_video(&video, &target)` (unchanged)

**Reconcile pass** — `ChannelVideoReconciler::reconcile_channel(channel, _)` / playlist equivalent
1. existing pass → after the internal reconcile, regardless of its result: `VideoCategoryAssigner::assign_source(&CategoryMember::Channel(channel.id))` (`lock()` → as `assign_video`, all the source's records); error ⇒ `warn!`; the pass returns the internal reconcile's result.

**Record removed** — `ChannelVideoReconciler` / `PlaylistVideoReconciler` removing a video, `ChannelDeleter::delete_channel_videos`, `PlaylistDeleter` equivalent
1. `VideoRepository::delete(&id)` → `VideoCategoriesRepository::delete(&id)` (next to the transcript delete).

**Category reads** — `CategoryVideoCollector::downloaded_videos(&category)`
1. `VideoCategoriesRepository::list_in_category(&category.id)` ⇒ record ids
2. per member, channels first then playlists, each in `category.members` order: indexed `list_for_channel` / `list_for_playlist`, keeping the records holding the category (in source order)
3. records not reached (their source left the category): `find_by_video` (channel, then playlist) ⇒ grouped by source, channels first, after the members
4. per source `ChannelRepository::find` / `PlaylistRepository::find` (gone ⇒ skipped) → `VideoRepository::find_many(ids)` → `filter(Video::is_downloaded)` → `SourcedVideo`.
   Callers (`CategoryViewSearcher`, `VideoSearcher::list_for_category` / `category_ids_by_video`, `VideoWatchStateUpdater::mark_category_watched`) unchanged.

## Test Plan

Behaviour tests (application layer):

`application/http/categories/mod.rs`
1. `it_should_categorize_the_videos_of_an_added_member` — set members [@chess] ⇒ its 2 records `VideoCategories([chess])`; one `VideoCategoriesChanged` with both ids.
2. `it_should_uncategorize_the_videos_of_a_removed_member` — @chess in [chess] removed ⇒ records `none`; event names them.
3. `it_should_keep_the_other_categories_of_a_member_removed_from_one` — @chess in chess+funny, removed from funny ⇒ `[chess]`.
4. `it_should_not_categorize_other_copies_of_a_member_video` — same YouTube id in an uncategorized playlist ⇒ that record stays `none`.
5. `it_should_uncategorize_stale_videos_of_a_former_member` — rows for a record whose source is no longer a member ⇒ cleared on set members.
6. `it_should_not_announce_a_members_set_that_changes_no_video` — same members again ⇒ no event.
7. `it_should_uncategorize_the_videos_of_a_deleted_category` — delete funny ⇒ @chess records `[chess]`; event.
8. `it_should_announce_the_videos_of_a_renamed_category` — rename ⇒ rows unchanged; event with its records.
9. `it_should_not_announce_a_recolored_category` — color only ⇒ no event.
10. Existing list/count/total, `list_for_category` (`http/videos`) and `mark_category_watched` suites stay green with seeds extended by `VideoCategories` (regression of identical results).

`application/tasks/sync_video_task.rs`
11. `it_should_categorize_a_new_video_of_a_categorized_channel` — Pending record of @chess in [chess] ⇒ `[chess]`; event.
12. `it_should_leave_a_new_video_of_an_uncategorized_playlist_uncategorized` — ⇒ `none`; no event.

`application/tasks/reconcile_channel_task.rs`
13. `it_should_repair_the_categories_of_stored_videos` — @chess in [chess], record with `none` ⇒ `[chess]`; event.
14. `it_should_leave_the_categories_of_an_uncategorized_channel` — record with rows, channel in no category ⇒ rows unchanged; no event.
15. `it_should_delete_the_categories_of_a_removed_video` — video dropped from channel ⇒ its rows gone.

`application/tasks/reconcile_playlist_task.rs`
16. `it_should_repair_the_categories_of_stored_playlist_videos`

`application/http/channels/mod.rs` / `playlists/mod.rs`
17. `it_should_delete_the_categories_of_a_deleted_channel_videos`
18. `it_should_delete_the_categories_of_a_deleted_playlist_videos`

Infrastructure tests:

Added by the audit:
- `http/categories`: `it_should_only_remove_a_deleted_category_from_videos_of_an_uncategorized_source`
- `http/videos`: `it_should_list_a_shared_video_once_from_the_first_member`, `it_should_list_a_shared_video_once_from_a_member_before_a_former_member`
- The `CategoryLock` serialization has no deterministic test (it would need timing-based threads).

`infrastructure/repositories/sqlite_video_categories_repository.rs`
19. `it_should_find_none_for_a_record_without_rows`
20. `it_should_replace_a_record_set_on_save`
21. `it_should_delete_all_rows_when_saving_an_empty_set`
22. `it_should_list_records_in_a_category_with_their_full_sets`
23. `it_should_delete_a_record_rows`

`infrastructure/shared/sqlite_migrations.rs`
24. `it_should_backfill_video_categories_from_members` — channel and playlist members, an uncategorized source, a record in two categories.

## Known Limitations

- `save_and_announce` saves the sets, then publishes `VideoCategoriesChanged` separately (the codebase's save-then-publish pattern). If the publish fails, every retry finds nothing changed, so that event is never re-sent. `plex-category-labels` must heal it: its periodic sync should also re-mark synced rows whose labels differ from the record's current categories.
