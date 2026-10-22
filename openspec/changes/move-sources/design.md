# Design

## Context

Video filenames are stored relative to the show directory
(`<videos root>/<source path>`), so a move is a directory rename plus a new
`path` on the source. The only other place the absolute show directory is
stored is the `output_dir` of queued per-video tasks (download, thumbnail,
subtitles, delete file), which the settled-source precondition rules out.
The on-demand reconcile runs inline in its HTTP handler, not through the
task queue, so the light lane alone cannot keep moves and reconciles apart.

## Goals / Non-Goals

**Goals:**
- One `ShowMover` service for both playlists and channels.
- An in-process per-show lock shared by the mover and both reconcilers.

**Non-Goals:**
- Cross-filesystem moves, copying, progress, persisted "moving" state.
- Rewriting queued task payloads (they are absent by precondition).

## Decisions

- **Rename, then record, roll back on failure.** Checks DB and FS, renames,
  then updates the source. A failed update renames back. A crash between the
  two is accepted (milliseconds wide).
- **In-memory lock, not persisted.** The critical section is milliseconds;
  a restart drops every lock and nothing is half done. Reconcilers `lock`
  (block), the mover `try_lock` (409), because a reconcile can hold it for
  the length of its YouTube calls.
- **Pending recurring reconcile left alone** rather than paused and
  re-added: pausing doesn't cover running or on-demand reconciles and
  risks losing the chain on a crash.
- **"Work in progress" = a video not Downloaded/Errored/Excluded, a
  non-completed task whose `output_dir` is the show directory, or a pending
  `video_downloaded` event whose `output_dir` is the show directory.** Every
  per-video task carries `output_dir`, including `delete_video_file`, which
  has no video id. The event counts because its subscriber schedules the
  subtitles fetch with that absolute directory: until it is handled (5 s
  poll, longer while it retries) a move would send the fetch to the old
  folder. The consumer schedules the task before deleting the event, so
  there is no gap between the two.
- **Pending deletes count as used paths.** A deleted source's folder is
  removed recursively later, by a `delete_{playlist,channel}_files` task
  scheduled from its `*Deleted` event. A destination that overlaps the path
  of a non-completed delete task or a pending delete event is
  `DestinationInUse`, or the delete would wipe the moved show.
- **`PlaylistPath` rejects `.` segments**, like `..`: `overlaps` compares
  segments as strings, so `a/./b` would slip past it. This applies on
  creation too; the web folder name and new-folder inputs reject `.` and
  `..`.
- **Listings report work in progress by the mover's own rule.** The
  settledness rule lives in one domain function (`domain/show/show_work.rs`)
  used by `ShowMover` and by both view searchers, so the edit dialog can
  disable "Move…" exactly when a move would be refused for work in
  progress. A running reconcile is not reported (it lasts seconds); its
  409 still reaches the dialog. The listings poll every 5 s while any
  source has work in progress (60 s otherwise), so "Move…" enables within
  seconds of the source settling.
- **Plex via a `ShowMoved` event** handled by a subscriber registered only
  when Plex is configured, reusing the outbox's async retry.

## Files

- `src/domain/show/show_key.rs`: `ShowKey`, the lock key (playlist or channel).
- `src/domain/show/errors.rs`: `MoveShowError`.
- `src/domain/show/events.rs`: `ShowMoved` event.
- `src/domain/show/show_work.rs`: `has_work_in_progress`, the settledness rule shared by the mover and the listings.
- `src/domain/show/mod.rs`: export the new modules.
- `src/domain/playlist/playlist_view.rs`, `src/domain/channel/channel_view.rs`: `work_in_progress` field.
- `src/domain/services/playlist_view_searcher.rs`, `channel_view_searcher.rs`: compute it (+ task repository, event repository, videos root).
- `src/application/http/{playlists,channels}/dto.rs`: `work_in_progress` on the list item responses.
- `src/domain/event/domain_event.rs`: `DomainEvent::ShowMoved` variant.
- `src/domain/playlist/playlist.rs`: `Playlist::with_path`.
- `src/domain/channel/channel.rs`: `Channel::with_path`.
- `src/domain/video/video.rs`: a test pinning the reused `Video::is_download_settled` (Downloaded, Errored or Excluded); no new predicate.
- `src/domain/playlist/playlist_path.rs`: `PlaylistPath::overlaps`; reject `.` segments (+ value object tests).
- `src/domain/task/task.rs`: `Task::output_dir_of`, mirrors `exclusivity_key`; `Task::deleted_path_of`.
- `src/domain/task/scheduled_task.rs`: `ScheduledTask::output_dir`, `ScheduledTask::deleted_path`.
- `src/domain/event/scheduled_event.rs`: `ScheduledEvent::output_dir` (`video_downloaded`), `ScheduledEvent::deleted_path` (`*Deleted`).
- `src/domain/services/show_mover.rs`: the move use case.
- `src/domain/services/mod.rs`: export `ShowMover`.
- `src/domain/services/playlist_video_reconciler.rs`: take the show lock in every entry point.
- `src/domain/services/channel_video_reconciler.rs`: same.
- `src/infrastructure/shared/show_locks.rs`: `ShowLocks`, the in-process lock registry.
- `src/infrastructure/shared/mod.rs`: export it.
- `src/infrastructure/repositories/filesystem_video_file_repository.rs`: `dir_exists`, `rename_dir` (+ fake).
- `src/infrastructure/infrastructure_container.rs`: one shared `Arc<ShowLocks>`.
- `src/application/http/playlists/{mod,dto}.rs`: `move_playlist` handler, `MovePlaylistRequest`.
- `src/application/http/channels/{mod,dto}.rs`: `move_channel` handler, `MoveChannelRequest`.
- `src/application/http/error.rs`: `move_show_error` mapping shared by both handlers.
- `src/application/http/mod.rs`: `show_mover` in `ApiServices`, two routes.
- `src/application/subscribers/scan_plex_folder_on_show_moved.rs`: Plex scan of both folders.
- `src/application/subscribers/mod.rs`: register it when Plex is configured.
- `src/serve.rs`: wire `ShowMover`, pass `ShowLocks` to reconcilers.
- `web/src/api/{client,types}.ts`: `movePlaylist`, `moveChannel`.
- `web/src/hooks/useSaveLocation.ts`: optional initial path; current path not "occupied"; don't remember parent on move; folder name `.`/`..` rejected.
- `web/src/components/FolderBrowser.tsx` (+ `SaveToField.test.tsx`): new-folder name `.`/`..` rejected.
- `web/src/components/MoveSourceDialog.tsx` (+ test): the move dialog.
- `web/src/components/LocationLine.tsx`: the "Location" line with "Move…", shared by both edit dialogs; "Move…" disabled with an explanation while the source has work in progress.
- `web/src/api/types.ts`, `web/src/test/helpers.tsx`: `work_in_progress` on `PlaylistListItem`/`ChannelListItem` and their builders.
- `web/src/components/EditPlaylistDialog.tsx` / `EditChannelDialog.tsx` (+ tests): Location line; each owns a `moving` state and renders `MoveSourceDialog` itself, so callers are unchanged.
- `smoke-tests/tests/moveSources.spec.js`: specs moving a playlist and a channel through the UI (route wiring).

## Types & Signatures

```rust
// domain/show/show_key.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShowKey { Playlist(PlaylistId), Channel(ChannelHandle) }

// domain/show/errors.rs
pub enum MoveShowError {
    NotFound,                       // 404
    Reconciling,                    // 409
    WorkInProgress,                 // 409
    SamePath,                       // 409
    DestinationInUse(PlaylistPath), // 409: equal to / nested with a source's path or a pending delete's path
    DestinationExists(PlaylistPath),// 409: on disk
    Filesystem(anyhow::Error),      // 500: rename failed (e.g. EXDEV)
    Repository(anyhow::Error),      // 500
}

// domain/show/events.rs
pub struct ShowMoved { pub from: String, pub to: String } // absolute show dirs
impl ShowMoved { pub const EVENT_TYPE: &str = "show_moved"; }

// domain/playlist/playlist.rs, domain/channel/channel.rs
pub fn with_path(self, path: PlaylistPath) -> Self;

// domain/playlist/playlist_path.rs
/// The same folder, or one inside the other, segment-wise.
pub fn overlaps(&self, other: &PlaylistPath) -> bool;
// (settledness reuses the existing Video::is_download_settled)

// domain/task/task.rs
/// The show directory a per-video task writes to or deletes from.
pub fn output_dir_of(task_type: &str, payload: &str) -> Option<String>;
/// The source path a delete_{playlist,channel}_files task removes.
pub fn deleted_path_of(task_type: &str, payload: &str) -> Option<String>;
// domain/task/scheduled_task.rs
pub fn output_dir(&self) -> Option<String>;
pub fn deleted_path(&self) -> Option<String>;
// domain/event/scheduled_event.rs
pub fn output_dir(&self) -> Option<String>;   // video_downloaded only
pub fn deleted_path(&self) -> Option<String>; // playlist_deleted / channel_deleted

// domain/show/show_work.rs
/// A video not Downloaded/Errored/Excluded, or a non-completed task or a
/// pending event whose output_dir is `show_dir`.
pub fn has_work_in_progress(videos: &[Video], tasks: &[ScheduledTask], events: &[ScheduledEvent], show_dir: &Path) -> bool;

// domain/playlist/playlist_view.rs, domain/channel/channel_view.rs
pub work_in_progress: bool,

// infrastructure/shared/show_locks.rs
#[derive(Default)]
pub struct ShowLocks { held: Mutex<HashSet<ShowKey>>, released: Condvar }
pub struct ShowLockGuard<'a> { locks: &'a ShowLocks, key: ShowKey } // Drop releases
impl ShowLocks {
    pub fn lock(&self, key: ShowKey) -> ShowLockGuard<'_>;            // blocks
    pub fn try_lock(&self, key: ShowKey) -> Option<ShowLockGuard<'_>>;
}

// infrastructure/repositories/filesystem_video_file_repository.rs (VideoFileRepository)
/// Any file or directory at the path.
fn dir_exists(&self, dir: &Path) -> bool;
/// Fails if `to` exists (POSIX rename would replace an empty dir), creates
/// `to`'s missing parents, then renames in place (never copies).
fn rename_dir(&self, from: &Path, to: &Path) -> anyhow::Result<()>;
// FakeVideoFileRepository: with_dir(..), with_failing_renames(), renamed_dirs()

// domain/services/show_mover.rs
pub struct ShowMover {
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    task_repository: Arc<dyn TaskRepository>,
    event_repository: Arc<dyn EventRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    show_locks: Arc<ShowLocks>,
    videos_root: PathBuf,
}
pub trait ShowMoverApi: Send + Sync {
    fn move_playlist(&self, id: PlaylistId, path: PlaylistPath) -> Result<Playlist, MoveShowError>;
    fn move_channel(&self, id: ChannelHandle, path: PlaylistPath) -> Result<Channel, MoveShowError>;
}

// application/http/{playlists,channels}/dto.rs
#[derive(Deserialize)] pub struct MovePlaylistRequest { pub path: Option<String> }
#[derive(Deserialize)] pub struct MoveChannelRequest  { pub path: Option<String> }
```

```ts
// web/src/api/client.ts
// mirror updatePlaylist/updateChannel's response types
export function movePlaylist(id: string, path: string): Promise<PlaylistListItem>
export function moveChannel(handle: string, path: string): Promise<CreatedChannel>
// web/src/hooks/useSaveLocation.ts
export function useSaveLocation(mode: SaveMode, nameSource: string, initialPath?: string): SaveLocation
// web/src/components/MoveSourceDialog.tsx
export function MoveSourceDialog(props: {
  kind: SaveMode; name: string; currentPath: string;
  onMove: (path: string) => Promise<unknown>; onClose: () => void
}): JSX.Element
```

## Call Stack

Move a playlist (channel identical, with `channel_*` ports and `ShowKey::Channel`):

```
POST /api/playlists/{id}/move {path}
  playlists::move_playlist(State<ShowMover>, Path(id), Json(req))
    PlaylistId::new(id)?, PlaylistPath::new(required(req.path, MISSING_PATH)?)?
    run_blocking -> ShowMover::move_playlist(id, path)
      show_locks.try_lock(ShowKey::Playlist(id))         -> None => Reconciling
      playlist_repository.find(&id)                      -> None => NotFound
      move_show(current, path, video_ids, record)         // private, shared with move_channel
      ensure_destination_free(&playlist.path, &path)
        path == current                                  => SamePath
        playlist_repository.list() + channel_repository.list()
        + task_repository.list_non_completed() -> task.deleted_path()
        + event_repository.list_eligible()     -> event.deleted_path()
          any used.overlaps(&path)                       => DestinationInUse
        video_file_repository.dir_exists(root/path)      => DestinationExists
      ensure_settled(videos, &old_dir) -> has_work_in_progress(..)
        playlist_video_repository.list_for_playlist(&id) -> video_repository.find_many(ids)
          any !video.is_download_settled()               => WorkInProgress
        task_repository.list_non_completed()
          any task.output_dir() == old_dir               => WorkInProgress
        event_repository.list_eligible()
          any event.output_dir() == old_dir              => WorkInProgress
      relocate(&old_dir, &new_dir, || playlist_repository.update(&playlist.with_path(path)))
        dir_exists(old_dir) ? rename_dir(old_dir, new_dir) : skip   (Err => Filesystem)
        update(..) Err => rename_dir(new_dir, old_dir); Repository
      event_publisher.publish(ShowMoved{from: old_dir, to: new_dir})   (Err => warn only)
      -> Ok(moved playlist)                             (guard dropped: unlocked)
    -> Json(PlaylistResponse)

Reconcile (recurring task / initial subscriber / on-demand handler)
  PlaylistVideoReconciler::{reconcile, initial_reconcile, force_reconcile}(id)
    let _guard = show_locks.lock(ShowKey::Playlist(id.clone()))   // waits for a move
    find_playlist(&id)  // re-read after the wait: sees the new path
    ...unchanged...

ShowMoved (outbox) -> ScanPlexFolderOnShowMoved::handle(payload)
  scanner.scan_folder(to)?; scanner.scan_folder(from)
```

## Test Plan

1. Behaviour tests (application layer)

   `application/http/playlists` — `move_playlist` (exhaustive, real SQLite, fake files):
   1. `it_should_move_a_playlist`: 200 with new path; repo has new path; fake recorded rename `/videos/a` -> `/videos/b`; `ShowMoved` in outbox.
   2. `it_should_keep_the_pending_reconcile_when_moving`: pending reconcile unchanged in `list_non_completed`.
   3. `it_should_move_a_playlist_with_errored_and_excluded_videos`.
   4. `it_should_record_the_new_path_if_the_show_directory_is_missing`: no rename recorded, path updated.
   5. `it_should_fail_to_move_if_path_missing`: 400.
   6. `it_should_fail_to_move_if_path_invalid`: 400 (one VO case).
   7. `it_should_fail_to_move_a_missing_playlist`: 404.
   8. `it_should_fail_to_move_to_the_same_path`: 409.
   9. `it_should_fail_to_move_to_a_path_used_by_a_channel`: 409.
   10. `it_should_fail_to_move_into_its_own_path`: 409 (`a` -> `a/b`).
   11. `it_should_fail_to_move_inside_another_playlist`: 409.
   12. `it_should_fail_to_move_if_destination_exists_on_disk`: 409, no rename.
   13. `it_should_fail_to_move_if_a_video_is_pending`: 409.
   14. `it_should_fail_to_move_if_a_thumbnail_fetch_is_queued`: 409 (all videos downloaded).
   15. `it_should_fail_to_move_while_reconciling`: lock held via `ShowLocks::lock` in test; 409.
   16. `it_should_fail_to_move_if_rename_fails`: failing fake; 500, path unchanged, no event.
   17. `it_should_rename_back_if_recording_the_path_fails`: playlist repository double whose `update` fails (the one place SQLite can't be made to fail); 500, renames recorded `a->b` then `b->a`, no event.
   - `it_should_fail_to_move_while_a_downloaded_video_is_not_yet_handled`: pending `video_downloaded` event in the show dir; 409, event still pending.
   - `it_should_fail_to_move_into_the_folder_of_a_source_being_deleted`: queued `delete_playlist_files` for `archive`, move to `archive/b`; 409, no rename.
   - `it_should_fail_to_move_into_the_folder_of_a_source_whose_deletion_is_not_yet_handled`: pending `playlist_deleted` event; 409.

   `application/http/channels` — `move_channel` (what differs + wiring):
   18. `it_should_move_a_channel`: 200, new path, rename recorded, `ShowMoved`.
   19. `it_should_fail_to_move_a_missing_channel`: 404.
   20. `it_should_fail_to_move_if_a_channel_video_is_pending`: 409 (channel video lookup wired).

   `application/tasks/reconcile_playlist_task.rs` / `reconcile_channel_task.rs`:
   21. `it_should_wait_for_a_move_before_reconciling`: hold lock on another thread, release after updating path; reconcile writes `tvshow.nfo` under the new path.
   22. Same for channels.

   `application/subscribers/scan_plex_folder_on_show_moved.rs`:
   23. `it_should_scan_the_new_and_old_folders`.
   24. `it_should_fail_if_plex_is_unreachable` (so the event is retried).

   Domain units:
   25. `task.rs`: `it_should_read_the_output_dir_of_every_per_video_task`, `it_should_have_no_output_dir_for_reconciles`.
   26. `video.rs`: `it_should_be_settled_only_when_downloaded_errored_or_excluded` (on `is_download_settled`).
   `playlist_path.rs`: overlap value object tests (same path, inside, containing, prefix-sharing sibling, sibling); `.` segment rejected (`.`, `./a`, `a/./b`, `a/.`).
   `task.rs`: `deleted_path_of` for source file deletions, none for other tasks or undecodable payloads.
   `scheduled_event.rs`: `output_dir` of `video_downloaded`, `deleted_path` of `*_deleted`, none for other events or undecodable payloads.

2. Infrastructure tests

   `infrastructure/shared/show_locks.rs`:
   27. `it_should_refuse_try_lock_while_held`.
   28. `it_should_release_on_drop`.
   29. `it_should_block_lock_until_released`.
   30. `it_should_not_contend_across_keys`.

   `infrastructure/repositories/filesystem_video_file_repository.rs` (temp dir):
   31. `it_should_rename_a_directory_creating_missing_parents`.
   32. `it_should_fail_to_rename_onto_an_existing_directory`.
   33. `it_should_report_whether_a_directory_exists`.

   Listings (`application/http/playlists` / `channels`, `list_*`):
   - `it_should_list_a_playlist_with_a_pending_video_as_work_in_progress`.
   - `it_should_list_a_playlist_with_a_queued_task_as_work_in_progress`.
   - `it_should_list_a_channel_with_a_pending_video_as_work_in_progress`.
   - `it_should_list_a_playlist_with_an_unhandled_downloaded_video_as_work_in_progress`.

3. Web (Vitest, `mockApi`)
   34. `MoveSourceDialog`: prefilled parent/folder; save disabled when unchanged; posts `POST /api/playlists/PL1/move` with new path and closes; shows server error and stays open; cancel sends nothing.
   35. `EditPlaylistDialog` / `EditChannelDialog`: shows Location; "Move…" opens the move dialog without saving; "Move…" disabled with the explanation while the source has work in progress.
   36. `MoveSourceDialog`: refuses a folder name of `.` / `..`; `SaveToField`: rejects a new folder named `.` / `..`.

4. Smoke: `moveSources.spec.js` waits for the listing's `work_in_progress` to be false before moving.
