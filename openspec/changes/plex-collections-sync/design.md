# Design — plex-collections-sync

## Files

- `src/domain/plex/mod.rs` — new `plex` module exports.
- `src/domain/plex/plex_item.rs` — `PlexItem` read model (a scanned library item).
- `src/domain/plex/plex_collection.rs` — `PlexCollection` read model.
- `src/domain/services/plex_collection_reconciler.rs` — new domain service: the global convergence pass.
- `src/domain/services/plex_collection_deleter.rs` — new domain service: deletes one collection by name.
- `src/domain/services/mod.rs` — export both services.
- `src/infrastructure/repositories/plex_collection_repository.rs` — `PlexCollectionRepository` port, `HttpPlexCollectionRepository` (reqwest with an explicit 10s timeout — the task shares the serial Light lane, so a hung Plex must not hold the slot; JSON via `Accept: application/json`, `X-Plex-Token` on every call) and `FakePlexCollectionRepository`, together like the youtube repositories.
- `src/infrastructure/repositories/mod.rs` — export the new repository.
- `src/application/tasks/reconcile_plex_collections_task.rs` — recurring global task handler + startup seeding, mirroring `update_ytdlp_task.rs`.
- `src/application/tasks/mod.rs` — export the task.
- `src/application/subscribers/delete_plex_collection_on_playlist_deleted.rs` — deletes the collection on `playlist_deleted`.
- `src/application/subscribers/delete_plex_collection_on_channel_deleted.rs` — deletes the collection on `channel_deleted`.
- `src/application/subscribers/mod.rs` — register both subscribers only when Plex is enabled.
- `src/domain/task/task.rs` — new `Task::ReconcilePlexCollections` variant (empty payload).
- `src/domain/event/domain_event.rs` — add `name` to `PlaylistDeleted` and `ChannelDeleted` (the deleted entity's display name; the subscriber can no longer look it up after deletion).
- `src/domain/services/playlist_deleter.rs`, `src/domain/services/channel_deleter.rs` — publish the deleted events with `name`.
- `src/infrastructure/repositories/task_executor.rs` — route `reconcile_plex_collections` tasks to the new handler.
- `src/serve.rs` — read `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN`, `YARRTUBE_PLEX_SECTION_ID`, `YARRTUBE_PLEX_RECONCILE_INTERVAL_SECONDS` (default 900); when the first three are set, wire the repository/services/task/subscribers and seed the recurring task.
- `README.md` — Plex integration setup (env vars, token, recommend "hide items which are in collections").

## Types & Signatures

```rust
// domain/plex/plex_item.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexItem {
    pub rating_key: String,
    pub youtube_video_id: String, // from the item's youtube://<id> Guid
}

// domain/plex/plex_collection.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexCollection {
    pub rating_key: String,
    pub title: String,
}

// infrastructure/repositories/plex_collection_repository.rs
pub trait PlexCollectionRepository: Send + Sync {
    fn list_items(&self) -> anyhow::Result<Vec<PlexItem>>;
    fn list_collections(&self) -> anyhow::Result<Vec<PlexCollection>>;
    fn list_collection_items(&self, collection_rating_key: &str) -> anyhow::Result<Vec<PlexItem>>;
    /// Creates the collection with the given members and sets its sort to
    /// alphabetical (POST /library/collections + collectionSort pref).
    fn create_collection(&self, title: &str, rating_keys: &[String]) -> anyhow::Result<()>;
    fn add_items(&self, collection_rating_key: &str, rating_keys: &[String]) -> anyhow::Result<()>;
    fn remove_item(&self, collection_rating_key: &str, rating_key: &str) -> anyhow::Result<()>;
    fn delete_collection(&self, collection_rating_key: &str) -> anyhow::Result<()>;
}

pub struct PlexConfig {
    pub base_url: String,
    pub token: String,
    pub section_id: String,
}

pub struct HttpPlexCollectionRepository { /* config + reqwest::blocking::Client, machine id cached after first fetch */ }
impl HttpPlexCollectionRepository {
    pub fn new(config: PlexConfig) -> Self;
}

pub struct FakePlexCollectionRepository { /* Mutex-held items + collections; failing() constructor */ }

// domain/services/plex_collection_reconciler.rs
pub struct PlexCollectionReconciler { /* playlist, channel, playlist_video, channel_video, video and plex_collection repositories */ }
impl PlexCollectionReconciler {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        plex_collection_repository: Arc<dyn PlexCollectionRepository>,
    ) -> Self;
}

pub trait PlexCollectionReconcilerApi: Send + Sync {
    /// One convergence pass over every playlist and channel. Per-collection
    /// failures are logged and skipped; only pass-wide failures (e.g. the
    /// section listing) return Err.
    fn reconcile_all(&self) -> anyhow::Result<()>;
}

// domain/services/plex_collection_deleter.rs
pub struct PlexCollectionDeleter { /* plex_collection_repository */ }
impl PlexCollectionDeleter {
    pub fn new(plex_collection_repository: Arc<dyn PlexCollectionRepository>) -> Self;
}

pub trait PlexCollectionDeleterApi: Send + Sync {
    /// Deletes the collection with this title, if any; missing is a no-op.
    fn delete(&self, name: &str) -> anyhow::Result<()>;
}

// application/tasks/reconcile_plex_collections_task.rs
pub struct ReconcilePlexCollectionsTask { /* reconciler, task_repository, clock, interval_seconds */ }
impl ReconcilePlexCollectionsTask {
    pub fn new(
        reconciler: PlexCollectionReconciler,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        interval_seconds: i64,
    ) -> Self;
}
impl TaskHandler for ReconcilePlexCollectionsTask {
    fn handle(&self, payload: &str, is_last_attempt: bool) -> anyhow::Result<()>;
}
/// Seeds the recurring task at startup unless a non-terminal one exists.
pub fn schedule_reconcile_plex_collections_if_absent(
    task_repository: &Arc<dyn TaskRepository>,
    clock: &Arc<dyn Clock>,
) -> anyhow::Result<()>;

// application/subscribers/delete_plex_collection_on_playlist_deleted.rs
pub struct DeletePlexCollectionOnPlaylistDeleted { /* deleter */ }
impl Subscriber for DeletePlexCollectionOnPlaylistDeleted {
    fn handle(&self, payload: &Value) -> anyhow::Result<()>; // reads payload["name"]
}
// delete_plex_collection_on_channel_deleted.rs is symmetric.

// domain/event/domain_event.rs (changed variants only)
PlaylistDeleted { playlist_id: String, name: String, path: String }
ChannelDeleted { channel_id: String, name: String, path: String }

// domain/task/task.rs
Task::ReconcilePlexCollections // kind "reconcile_plex_collections", empty payload
// Runs in TaskLane::Light via lane_for's default arm — deliberate, no new
// mapping: network-only work against the local Plex server, same weight
// class as the reconcile tasks already in that lane.
```

## Call Stack

Recurring reconcile pass:

```
TaskExecutor ("reconcile_plex_collections")
  ReconcilePlexCollectionsTask::handle(payload, is_last_attempt)
    PlexCollectionReconciler::reconcile_all()
      plex_collection_repository.list_items()            -> youtube_video_id -> rating_key map
      plex_collection_repository.list_collections()      -> title -> rating_key map
      playlist_repository.list() / channel_repository.list()
      for each playlist/channel (errors logged, loop continues):
        desired youtube ids:
          playlist_video_repository.list_for_playlist(id) | channel_video_repository.list_for_channel(id)
          video_repository.find_many(video record ids), keep status == Downloaded
        desired rating keys = desired ids ∩ scanned map
        if no collection for name && desired non-empty:
          plex_collection_repository.create_collection(name, desired_rating_keys)
        else if collection exists:
          members = plex_collection_repository.list_collection_items(collection_key)
          plex_collection_repository.add_items(collection_key, desired \ members)
          plex_collection_repository.remove_item(collection_key, m) for m in members \ desired
    task_repository.schedule(&Task::ReconcilePlexCollections, now + interval)   // always, like UpdateYtdlp
```

Collection deletion:

```
DomainEventsConsumer ("playlist_deleted" / "channel_deleted")
  DeletePlexCollectionOn{Playlist,Channel}Deleted::handle(payload)
    PlexCollectionDeleter::delete(payload["name"])
      plex_collection_repository.list_collections()
      plex_collection_repository.delete_collection(key)   // skip if title absent
```

Startup (serve.rs, only when `YARRTUBE_PLEX_URL` + `YARRTUBE_PLEX_TOKEN` + `YARRTUBE_PLEX_SECTION_ID` set):

```
serve
  HttpPlexCollectionRepository::new(PlexConfig)
  PlexCollectionReconciler::new(...) / PlexCollectionDeleter::new(...)
  register ReconcilePlexCollectionsTask in TaskExecutor
  register both delete subscribers in subscribers::registry
  schedule_reconcile_plex_collections_if_absent(task_repository, clock)
```

## Test Plan

Behaviour tests — `reconcile_plex_collections_task.rs` (primary adapter; real SQLite repositories, `FakePlexCollectionRepository`, `FixedClock`):

1. `it_should_create_a_collection_for_a_playlist_with_scanned_videos` — seeded playlist with downloaded videos present in the fake's items; asserts collection created with those rating keys and next task scheduled.
2. `it_should_create_a_collection_for_a_channel_with_scanned_videos` — same via a channel.
3. `it_should_skip_creating_a_collection_if_no_video_is_scanned_yet` — downloaded videos absent from Plex items; asserts no collection created.
4. `it_should_ignore_videos_that_are_not_downloaded` — pending/errored videos never reach the collection even when scanned.
5. `it_should_add_newly_scanned_videos_to_an_existing_collection` — existing collection missing one scanned member; asserts only the missing key is added.
6. `it_should_remove_videos_no_longer_tracked_from_the_collection` — collection holds a member whose video is gone from yarrtube state; asserts it is removed.
7. `it_should_do_nothing_if_already_in_sync` — second pass over converged state; asserts fake recorded no mutations.
8. `it_should_continue_reconciling_remaining_collections_if_one_fails` — fake fails one collection's create; asserts the other playlist's collection still reconciled and the task Ok.
9. `it_should_reschedule_the_next_reconcile_if_the_pass_fails` — fake `failing()` on `list_items`; asserts Err and next task scheduled.

Behaviour tests — `delete_plex_collection_on_playlist_deleted.rs`:

10. `it_should_delete_the_collection_if_a_playlist_is_deleted` — event with `name` matching an existing collection; asserts deletion.
11. `it_should_skip_if_no_collection_matches_the_playlist_name`.

Behaviour tests — `delete_plex_collection_on_channel_deleted.rs`:

12. `it_should_delete_the_collection_if_a_channel_is_deleted` — proves wiring; other cases covered by 10–11.

Behaviour tests — deleted-event payloads (in the existing delete adapters' test modules):

13. `it_should_publish_the_playlist_name_on_deletion` — asserts the outbox `playlist_deleted` payload includes `name`.
14. `it_should_publish_the_channel_name_on_deletion` — same for `channel_deleted`.

Infrastructure tests — `HttpPlexCollectionRepository` against mockito:

15. `it_should_list_section_items_with_their_youtube_ids` — parses items, keeps only those with a `youtube://` guid.
16. `it_should_list_collections`.
17. `it_should_list_collection_items`.
18. `it_should_create_a_collection_with_alphabetical_sort` — asserts machine id fetched from `/identity`, POST with `sectionId`/`title`/`uri`, then the `collectionSort` pref call.
19. `it_should_add_items_to_a_collection`.
20. `it_should_remove_an_item_from_a_collection`.
21. `it_should_delete_a_collection`.
22. `it_should_fail_if_the_server_replies_with_an_error` — non-2xx maps to Err.
