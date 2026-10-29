# Design

## Files

- `migrations/0015_plex_labels.sql` — `episode_labels(video_id PK, dirty, rating_key, section_id, labels JSON, checked_at)` and `category_playlists(section_id, label COLLATE NOCASE, rating_key, title, PK(section_id, label))`. No backfill: the periodic sync backfills.
- `migrations/0016_episode_labels_generation.sql` — `episode_labels.generation INTEGER NOT NULL DEFAULT 0`: times a row was marked dirty, so a sync's save skips a row marked since it read it.
- `src/infrastructure/shared/sqlite_migrations.rs` — registers 0015 and 0016.
- `Cargo.toml` — `percent-encoding`, to escape labels as Plex expects inside a comma-joined removal and a nested playlist URI.
- `src/domain/plex/plex_section.rs` — `PlexSection` gains `title` and `serves(root)`.
- `src/domain/plex/plex_episode.rs` — new `PlexEpisode`: an episode as listed by Plex (rating key + file).
- `src/domain/plex/plex_playlist.rs` — new `PlexPlaylist`: a playlist as listed by Plex.
- `src/domain/plex/plex_labels.rs` — new `PlexLabels` VO: a label set compared ignoring case.
- `src/domain/plex/mod.rs` — exports them.
- `src/domain/plex_label/episode_labels.rs` — new `EpisodeLabels` entity: a record's Plex label state (dirty, last written).
- `src/domain/plex_label/category_playlist.rs` — new `CategoryPlaylist` entity: a smart playlist yarrtube owns.
- `src/domain/plex_label/mod.rs`, `src/domain/mod.rs` — declare the module.
- `src/domain/video/video_owner.rs` — `VideoOwner` moved out of `video_reconciler.rs` and made public: a video's source, target and playlist position.
- `src/domain/video/mod.rs` — exports it.
- `src/domain/services/video_owner_finder.rs` — new `VideoOwnerFinder`: `VideoReconciler::find_owner` / `find_channel_owner` / `find_playlist_owner` / `target` extracted unchanged.
- `src/domain/services/video_reconciler.rs` — holds a `VideoOwnerFinder` instead of the channel/playlist/channel-video/playlist-video repositories it used only for the owner lookup (check each is otherwise unused before removing).
- `src/domain/services/plex_label_syncer.rs` — new `PlexLabelSyncer`: marks records, requests syncs, runs a sync (labels then playlists).
- `src/domain/services/mod.rs` — exports both.
- `src/domain/task/task.rs` — `Task::SyncPlexLabels` (`sync_plex_labels`, empty payload), lane `Plex`.
- `src/domain/task/task_lane.rs` — `TaskLane::Plex` (capacity 1 via the executor's default).
- `src/infrastructure/repositories/plex_library_repository.rs` — port gains identity, episodes, labels and playlist calls; `FakePlexLibraryRepository` gains episodes/labels/playlists state.
- `src/infrastructure/repositories/sqlite_episode_labels_repository.rs` — `EpisodeLabelsRepository` port + SQLite adapter.
- `src/infrastructure/repositories/sqlite_category_playlist_repository.rs` — `CategoryPlaylistRepository` port + SQLite adapter.
- `src/infrastructure/repositories/mod.rs`, `src/infrastructure/infrastructure_container.rs` — expose/build both.
- `src/application/tasks/sync_plex_labels_task.rs` — new `SyncPlexLabelsTask` handler.
- `src/application/tasks/mod.rs` — registers `sync_plex_labels` when a syncer is given.
- `src/application/subscribers/sync_plex_labels_on_video_categories_changed.rs` — marks the event's records, requests a sync now.
- `src/application/subscribers/sync_plex_labels_on_video_downloaded.rs` — marks the record, requests a sync in a minute.
- `src/application/subscribers/mod.rs` — `registry` gains `plex_label_syncer: Option<PlexLabelSyncer>` next to `plex_folder_scanner`; registers both subscribers when it is `Some`.
- `src/serve.rs` — `PlexIntegration: Clone`; builds `VideoOwnerFinder` (shared by `video_reconciler` and the syncer) and `PlexLabelSyncer`; passes the syncer to the task registry and `event_subscribers`; `prepare_task_queue` gains `Option<&PlexLabelSyncer>` and requests a sync at startup when it is `Some`.
- `web/src/lib/tasks.ts` (+ `tasks.test.ts`) — describes `sync_plex_labels` ("Syncing Plex category labels"), family `other`.
- `doc/PLEX.md` — labels, playlists, limits.
- Test-only wiring for the `VideoOwnerFinder` extraction (tests build `VideoReconciler` with a `VideoOwnerFinder`): `src/application/http/channels/mod.rs`, `src/application/http/playlists/mod.rs`, `src/application/subscribers/reconcile_on_channel_created.rs`, `src/application/subscribers/reconcile_on_playlist_created.rs`, `src/application/tasks/reconcile_channel_task.rs`, `src/application/tasks/reconcile_playlist_task.rs`, `src/application/tasks/sync_video_task.rs`, `src/domain/services/internal_video_reconciler.rs`.

## Types & Signatures

```rust
// domain/plex/plex_section.rs
pub struct PlexSection { pub id: String, pub title: String, pub locations: Vec<String> }
impl PlexSection {
    pub fn contains(&self, path: &str) -> bool; // unchanged
    /// Whether a location is under `root` or `root` is under a location.
    pub fn serves(&self, root: &str) -> bool;
}

// domain/plex/plex_episode.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexEpisode { pub rating_key: String, pub section_id: String, pub file: String }

// domain/plex/plex_playlist.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlexPlaylist { pub rating_key: String, pub title: String }

// domain/plex/plex_labels.rs
/// Sorted, each once ignoring case; `==` ignores case. Plex capitalizes the
/// first letter of a new label, so stored and read names may differ in case.
#[derive(Debug, Clone)]
pub struct PlexLabels(Vec<String>);
impl PlexLabels {
    pub fn new(names: Vec<String>) -> Self;
    pub fn none() -> Self;
    pub fn names(&self) -> &[String];
    /// Names in `self` missing from `other`, ignoring case.
    pub fn missing_from(&self, other: &PlexLabels) -> Vec<String>;
}
impl PartialEq for PlexLabels; impl Eq for PlexLabels;

// domain/plex_label/episode_labels.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeLabels {
    pub video_id: VideoRecordId,
    pub dirty: bool,
    /// The episode last written; None until first synced.
    pub rating_key: Option<String>,
    pub section_id: Option<String>,
    /// What yarrtube last wrote.
    pub labels: PlexLabels,
    /// Last time a sync looked at it; dirty rows are taken least recently
    /// checked first, so rows waiting on a download or import never starve the rest.
    pub checked_at: Option<DateTime<Utc>>,
    /// How many times it was marked dirty; a sync's save lands only if no
    /// mark came after the sync read it.
    pub generation: i64,
}
impl EpisodeLabels {
    pub fn pending(video_id: VideoRecordId) -> Self;      // dirty, never synced, generation 0
    pub fn mark_dirty(self) -> Self;                     // dirty, generation + 1
    /// Dirty (generation + 1) and no longer tied to an episode: Plex stopped
    /// listing the one last written, so its section and labels count no more.
    pub fn forget_episode(self) -> Self;
    /// Still dirty (not downloaded / not imported / Plex rejected), looked at `now`.
    pub fn checked(self, now: DateTime<Utc>) -> Self;
    pub fn synced(self, episode: &PlexEpisode, labels: PlexLabels, now: DateTime<Utc>) -> Self;
}

// domain/video/video_owner.rs (moved from video_reconciler.rs, now pub)
pub struct VideoOwner {
    pub source: CategoryMember,
    pub target: VideoTarget,
    /// The video's playlist position; `None` for a channel video.
    pub sort_position: Option<i64>,
}

// domain/services/video_owner_finder.rs
pub struct VideoOwnerFinder {
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    videos_path: String,
}
pub trait VideoOwnerFinderApi: Send + Sync {
    /// The channel owning the video, else the playlist; `None` if neither
    /// (or its source) is stored. Body moved verbatim from `VideoReconciler`.
    fn find(&self, video: &Video) -> anyhow::Result<Option<VideoOwner>>;
}
impl VideoOwnerFinderApi for VideoOwnerFinder; // the repo's `<Service>Api` convention

// domain/plex_label/category_playlist.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryPlaylist {
    pub section_id: String,
    pub label: String,     // category name = label; key ignores case
    pub rating_key: String,
    pub title: String,
}

// infrastructure/repositories/plex_library_repository.rs — added to the port
pub trait PlexLibraryRepository: Send + Sync {
    fn list_sections(&self) -> anyhow::Result<Vec<PlexSection>>;           // + title
    fn scan_path(&self, section_id: &str, path: &str) -> anyhow::Result<()>;
    /// GET /identity → machineIdentifier.
    fn server_id(&self) -> anyhow::Result<String>;
    /// GET /library/sections/{id}/all?type=4, paged; one per Media/Part file.
    fn list_episodes(&self, section_id: &str) -> anyhow::Result<Vec<PlexEpisode>>;
    /// GET /library/metadata/{rating_key} → Label[].tag.
    fn find_labels(&self, rating_key: &str) -> anyhow::Result<PlexLabels>;
    /// PUT /library/sections/{section}/all?type=4&id={rk}
    ///   &label[i].tag.tag=<each of labels>&label[].tag.tag-=<removed, each percent-encoded, comma-joined>
    ///   &label.locked=1 (0 when labels is empty).
    fn save_labels(&self, episode: &PlexEpisode, labels: &PlexLabels, removed: &[String]) -> anyhow::Result<()>;
    /// GET /playlists?playlistType=video.
    fn list_playlists(&self) -> anyhow::Result<Vec<PlexPlaylist>>;
    /// POST /playlists?type=video&smart=1&title=..&uri=server://{server_id}/com.plexapp.plugins.library
    ///   /library/sections/{section}/all?type=4&episode.label={label percent-encoded, %20 for a space}&episode.unwatched=1
    ///   &sort=originallyAvailableAt:desc
    fn create_smart_playlist(&self, server_id: &str, section_id: &str, label: &str, title: &str) -> anyhow::Result<PlexPlaylist>;
    /// DELETE /playlists/{rating_key}.
    fn delete_playlist(&self, rating_key: &str) -> anyhow::Result<()>;
}

// infrastructure/repositories/sqlite_episode_labels_repository.rs
pub trait EpisodeLabelsRepository: Send + Sync {
    /// Only the ids that have a row.
    fn find_many(&self, ids: &[VideoRecordId]) -> anyhow::Result<Vec<EpisodeLabels>>;
    /// Dirty rows, never-checked first then least recently checked, at most `limit`.
    fn list_dirty(&self, limit: usize) -> anyhow::Result<Vec<EpisodeLabels>>;
    /// Rows with a rating key.
    fn list_synced(&self) -> anyhow::Result<Vec<EpisodeLabels>>;
    /// Upsert, one transaction; a stored row with a newer `generation` (marked
    /// since the caller read it) is left as is.
    fn save_many(&self, rows: &[EpisodeLabels]) -> anyhow::Result<()>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

// infrastructure/repositories/sqlite_category_playlist_repository.rs
pub trait CategoryPlaylistRepository: Send + Sync {
    fn list(&self) -> anyhow::Result<Vec<CategoryPlaylist>>;
    fn save(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()>; // upsert by (section, label)
    fn delete(&self, playlist: &CategoryPlaylist) -> anyhow::Result<()>;
}

// domain/services/plex_label_syncer.rs
pub const SYNC_INTERVAL: Duration = 15 min;      // periodic heal
pub const AFTER_DOWNLOAD_DELAY: Duration = 60 s; // let Plex's async scan import it
/// A reconcile pass publishes one `VideoCategoriesChanged` per repaired video;
/// the first one schedules the sync this far out and `bring_forward` keeps it
/// there, so the pass's events collapse into one sync.
pub const CATEGORY_CHANGE_DELAY: Duration = 10 s;
pub const BATCH_SIZE: usize = 500;               // a full batch with progress ⇒ next sync requested now

pub struct PlexLabelSyncer {
    plex_library_repository: Arc<dyn PlexLibraryRepository>,
    episode_labels_repository: Arc<dyn EpisodeLabelsRepository>,
    category_playlist_repository: Arc<dyn CategoryPlaylistRepository>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    category_repository: Arc<dyn CategoryRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    videos_path: String,
    plex_videos_path: String,
}

pub trait PlexLabelSyncerApi: Send + Sync {
    /// Marks the records dirty (creating their rows) and requests a sync at now + `delay`.
    fn mark(&self, video_ids: &[VideoRecordId], delay: Duration) -> anyhow::Result<()>;
    /// A pending sync moves forward to `run_at` if due later, else one is scheduled.
    fn request_sync(&self, run_at: DateTime<Utc>) -> anyhow::Result<()>;
    /// One sync: heal drift, label a batch of dirty records, reconcile playlists,
    /// then request the next sync (now if the batch was full and at least one of its
    /// rows was synced or deleted, else now + SYNC_INTERVAL).
    /// Plex unreachable ⇒ logs, keeps everything dirty, still requests the next sync, Ok.
    fn sync(&self) -> anyhow::Result<()>;
}
```

Not part of `VideoReconciler::reconcile`, although it owns a video's desired state: labelling needs Plex's episode listing per library (one paged listing per sync, not per video), writes in per-section batches driven by dirty rows, and must run in `TaskLane::Plex` so a slow Plex never holds a video sync. So it is a sweep that `VideoDownloaded` and `VideoCategoriesChanged` trigger; the video sync only feeds it indirectly, through the `VideoCategoriesChanged` it publishes when it repairs a video's categories.

Playlist scope (open question, default chosen): a playlist exists only in libraries where its category has labelled episodes; titles get ` · <library title>` only when a category has playlists in more than one library. Playlist changes are delete + recreate (a renamed category is a new label, so a new filter); in-place `PUT /playlists/{id}` retitling is a later optimisation.

## Call Stack

**Category change** — `VideoCategoriesChanged { video_ids }`
1. `SyncPlexLabelsOnVideoCategoriesChanged::handle(payload)` → ids → `VideoRecordId::new`
2. `PlexLabelSyncer::mark(&ids, CATEGORY_CHANGE_DELAY)` → `EpisodeLabelsRepository::find_many(ids)` → existing `mark_dirty()` ∪ missing `EpisodeLabels::pending(id)` → `save_many` → `request_sync(now + CATEGORY_CHANGE_DELAY)`
3. `request_sync(run_at)` → `TaskRepository::find_pending(&Task::SyncPlexLabels)` → `Some` ⇒ `update(pending.bring_forward(run_at, now))`; `None` ⇒ `schedule(&Task::SyncPlexLabels, run_at)`

**Video sync** — `VideoReconciler::reconcile(&id)` → `VideoOwnerFinder::find(&video)` (was the private `find_owner`; same behaviour)

**Download** — `VideoDownloaded { video_id, .. }` (alongside the scan subscriber and `SyncVideoOnVideoEvent`)
1. `SyncPlexLabelsOnVideoDownloaded::handle` → `PlexLabelSyncer::mark(&[id], AFTER_DOWNLOAD_DELAY)`

**Startup** (Plex configured) — `serve::prepare_task_queue(.., Some(&syncer))` → `PlexLabelSyncer::request_sync(now)`

**Sync** — executor (lane `Plex`) → `SyncPlexLabelsTask::handle(payload, _)` → `Task::decode_sync_plex_labels_payload` → `PlexLabelSyncer::sync()`
1. `list_sections()` → keep `serves(plex_videos_path)`; `Err` ⇒ `warn!`, `request_sync(now + SYNC_INTERVAL)`, `Ok(())`
2. `list_episodes(section)` per kept section ⇒ `by_file: file → PlexEpisode`, `keys: rating_key set`
3. Heal drift:
   - `list_synced()` → rows whose `rating_key` ∉ `keys` ⇒ `forget_episode()` (an episode gone for good keeps no playlist alive; a re-imported one is labelled again)
   - every record in a category (`CategoryRepository::list` → `VideoCategoriesRepository::list_in_category` per category) with no row (`find_many`) ⇒ `EpisodeLabels::pending`
   - `save_many(healed)`
4. `list_dirty(BATCH_SIZE)` → `VideoCategoriesRepository::find_many(ids)` → names via `CategoryRepository::list()` ⇒ desired `PlexLabels` per record; per row:
   - `VideoRepository::find` → `None` ⇒ `EpisodeLabelsRepository::delete`; not downloaded ⇒ `checked(now)`
   - `VideoOwnerFinder::find(&video)` → `None` ⇒ `checked(now)`; else `owner.target.output_dir.join(filename)` → `plex_folder_path(videos_path, plex_videos_path, file)` → `by_file` → `None` ⇒ `checked(now)` (not imported yet)
   - current = `find_labels(&episode.rating_key)`, always (also catches labels edited by hand in Plex)
   - current == desired ⇒ no call; else `save_labels(&episode, &desired, &current.missing_from(&desired))`; `Err` ⇒ `warn!`, `checked(now)`, continue
   - ok ⇒ `row.synced(&episode, desired, now)`
   - `save_many(rows)` — rows marked since `list_dirty` read them (a category change during the sync) are not overwritten and stay dirty
5. Playlists:
   - desired = distinct `(section_id, category name)` over `list_synced()` labels matching a current category ignoring case (spelled as the category; a label no category carries counts for none), titled by that name (`· <section title>` if it spans sections)
   - `list_playlists()` → stored = `CategoryPlaylistRepository::list()`
   - stored not desired, or desired with a title change, ⇒ `delete_playlist` (ignore 404) + `delete`
   - desired without a stored row whose `rating_key` is in Plex ⇒ `create_smart_playlist(server_id(), ..)` + `save`
6. batch was full (`BATCH_SIZE` rows) and at least one row was synced or deleted ⇒ `request_sync(now)`, else `request_sync(now + SYNC_INTERVAL)` (a full batch of rows not labellable yet never makes syncs run back to back)

## Test Plan

Behaviour tests (application layer):

`application/tasks/sync_plex_labels_task.rs` (real SQLite, `FakePlexLibraryRepository`, `FixedClock`)
1. `it_should_label_the_episode_of_a_categorized_video` — dirty record in [Chess], episode in fake ⇒ fake labels ["Chess"], row synced with rating key.
2. `it_should_label_an_episode_with_every_category` — [Chess, Funny] ⇒ both.
3. `it_should_clear_the_labels_of_an_uncategorized_video` — fake has ["Chess"], record in none ⇒ [] and unlocked.
4. `it_should_not_rewrite_labels_already_in_sync` — fake "Chess", desired "chess" ⇒ no mutation.
5. `it_should_replace_the_label_of_a_renamed_category` — row labels ["Funny"], category now "Humor" ⇒ ["Humor"], "Funny" removed.
6. `it_should_keep_a_video_dirty_until_plex_imports_it` — no episode for the file ⇒ no mutation, row dirty.
7. `it_should_keep_a_video_dirty_until_it_is_downloaded`
8. `it_should_forget_a_deleted_video` — dirty row, no video ⇒ row gone.
9. `it_should_label_categorized_videos_never_synced` — no row, record in [Chess], downloaded ⇒ labelled (backfill).
10. `it_should_relabel_an_episode_plex_reimported` — synced row with a rating key no longer listed, new key for the file ⇒ labelled under the new key.
11. `it_should_label_the_others_if_plex_rejects_one` — fake failing for one rating key ⇒ other labelled; rejected stays dirty.
12. `it_should_keep_videos_dirty_if_plex_is_unreachable` — `failing()` ⇒ rows dirty, next sync pending at now + 15 min.
13. `it_should_schedule_the_next_sync` — nothing dirty ⇒ one `sync_plex_labels` pending at now + 15 min.
14. `it_should_schedule_the_next_sync_now_if_the_batch_was_full` — BATCH_SIZE labellable dirty rows ⇒ next sync at now.
14b. `it_should_not_sync_again_right_away_if_no_row_of_a_full_batch_could_be_labelled` — BATCH_SIZE dirty rows of videos not downloaded ⇒ next sync at now + 15 min.
15. `it_should_rotate_videos_waiting_for_plex` — a full batch of not-imported rows, then one importable row ⇒ the next sync labels it.
16. `it_should_create_a_smart_playlist_for_a_category` — labelled "Chess" in one section ⇒ fake playlist "Chess" with the label/section filter; stored row.
17. `it_should_title_playlists_by_library_if_a_category_spans_libraries` — two sections ⇒ "Chess · Subscriptions", "Chess · Sergi's Playlists".
18. `it_should_delete_the_playlist_of_a_category_without_labelled_episodes`
19. `it_should_replace_the_playlist_of_a_renamed_category`
20. `it_should_recreate_a_playlist_deleted_in_plex`
21. `it_should_not_touch_playlists_it_did_not_create`
21b. `it_should_keep_a_playlist_already_in_sync` — a second sync makes no Plex playlist mutation.
21c. `it_should_delete_the_playlist_of_a_category_whose_episodes_left_plex` — synced row whose rating key Plex no longer lists ⇒ its playlist deleted.
21d. `it_should_keep_one_playlist_for_a_category_renamed_by_case` — rows holding "chess" and "Chess", category "Chess" ⇒ one playlist "Chess" after two syncs.

`application/subscribers/sync_plex_labels_on_video_categories_changed.rs`
22. `it_should_mark_the_videos_and_request_a_sync_shortly` — rows dirty; one `sync_plex_labels` pending at now + 10 s.
23. `it_should_bring_forward_a_pending_sync` — pending at now + 15 min ⇒ moved to now + 10 s, still one.
23b. `it_should_collapse_category_changes_into_one_sync` — two events for different videos ⇒ both rows dirty, one `sync_plex_labels` pending at now + 10 s.

`application/subscribers/sync_plex_labels_on_video_downloaded.rs`
24. `it_should_mark_the_video_and_request_a_sync_in_a_minute`

`serve.rs`
25. `it_should_request_the_label_sync_at_startup_only_if_plex_is_configured` — `prepare_task_queue` with and without a syncer.

Regression (no new test): the existing `sync_video_task.rs`, `reconcile_channel_task.rs` and `reconcile_playlist_task.rs` suites pass unchanged after extracting `VideoOwnerFinder`.

Domain unit:
- `domain/task/task.rs`: 26. `it_should_run_label_syncs_in_the_plex_lane`, 26b. `it_should_reject_a_malformed_sync_plex_labels_payload`
- `domain/plex/plex_labels.rs`: 27. `it_should_equal_labels_ignoring_case`, 28. `it_should_list_labels_missing_from_another_set_ignoring_case`, 29. `it_should_keep_each_label_once_ignoring_case`
- `domain/plex/plex_section.rs`: 30. `it_should_serve_a_root_under_or_above_its_location`

Web (`web/src/lib/tasks.test.ts`)
31. `describes a Plex label sync in plain language`
32. `puts Plex label syncs in the other family`

Infrastructure tests:

`infrastructure/repositories/plex_library_repository.rs` (mockito)
33. `it_should_list_sections_with_their_titles`
34. `it_should_read_the_server_id`
35. `it_should_list_a_section_episodes_with_their_files` — two pages, multi-part media.
36. `it_should_read_an_episode_labels`
37. `it_should_save_labels_removing_the_others` — exact query: indexed adds, `-=` removals (each escaped, `Other%20one`), `label.locked=1`.
38. `it_should_clear_labels_and_unlock_them`
38b. `it_should_escape_each_removed_label_so_a_comma_never_splits_one` — each removed label percent-encoded (python-plexapi style) before the comma join.
39. `it_should_list_playlists`
40. `it_should_create_a_smart_playlist_filtered_by_label` — exact `uri`, the label escaped with `%20` for a space (as python-plexapi), never `+`.
41. `it_should_delete_a_playlist`

`infrastructure/repositories/sqlite_episode_labels_repository.rs`
42. `it_should_round_trip_an_episode_labels_row`
43. `it_should_list_dirty_rows_least_recently_checked_first_up_to_a_limit`
44. `it_should_list_only_synced_rows`
44b. `it_should_keep_a_row_marked_since_it_was_read` — a save carrying an older `generation` leaves the stored row as is.

`infrastructure/repositories/sqlite_category_playlist_repository.rs`
45. `it_should_save_list_and_delete_category_playlists`
46. `it_should_key_playlists_by_label_ignoring_case`
