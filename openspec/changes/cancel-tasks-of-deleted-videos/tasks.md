# Tasks

## 1. Walking skeleton

- [x] 1.1 Add `TaskRepository::delete_pending_for_video(&self, video_id: &VideoId)` to the trait with a stub `SqliteTaskRepository` body returning `Ok(())`; call it per video in `PlaylistDeleter::delete_playlist_videos` and `ChannelDeleter::delete_channel_videos`. Done when `cargo build` succeeds and all existing tests pass

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_remove_the_pending_tasks_of_a_deleted_playlists_videos`: deleting a playlist removes its video's pending `download_video` (future run_at), `fetch_thumbnail` and `summarize_video`
- [x] 2.2 `it_should_keep_the_pending_tasks_of_another_playlists_videos`: deleting playlist A keeps playlist B's video's pending `download_video`
- [x] 2.3 `it_should_remove_the_pending_tasks_of_a_deleted_channels_videos`: deleting a channel removes its video's pending tasks
- [ ] 2.4 `it_should_keep_the_pending_tasks_of_another_channels_videos`: deleting channel A keeps channel B's video's pending `download_video`

## 3. Infrastructure adapters (TDD)

### SqliteTaskRepository

- [ ] 3.1 `it_should_delete_every_pending_task_of_a_video`: the video's pending `download_video` and `fetch_thumbnail` are gone
- [ ] 3.2 `it_should_not_delete_a_running_task_of_a_video`: a claimed `download_video` of the video is still non-completed
- [ ] 3.3 `it_should_not_delete_the_pending_tasks_of_another_video`: another video's pending `download_video` and a `reconcile_playlist` are kept

## 4. Verification

- [ ] 4.1 Run `scripts/run-tests.sh --only web --only rust`; fmt, clippy and tests pass
