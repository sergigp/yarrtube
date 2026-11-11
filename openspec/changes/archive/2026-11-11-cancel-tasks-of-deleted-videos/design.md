# Design

## Files

- `src/infrastructure/repositories/sqlite_task_repository.rs`: new `TaskRepository::delete_pending_for_video`; the task table's owner.
- `src/domain/services/playlist_deleter.rs`: calls it per deleted video in `delete_playlist_videos`.
- `src/domain/services/channel_deleter.rs`: calls it per deleted video in `delete_channel_videos`.
- `src/application/http/playlists/mod.rs`, `src/application/http/channels/mod.rs`: behaviour tests (existing delete tests live here).

## Types & Signatures

```rust
pub trait TaskRepository: Send + Sync {
    // ...existing methods...

    /// Deletes every pending task whose payload names `video_id`. Running
    /// tasks are kept: they settle on their own once the video is gone.
    fn delete_pending_for_video(&self, video_id: &VideoId) -> anyhow::Result<()>;
}
```

```sql
DELETE FROM tasks WHERE json_extract(payload, '$.video_id') = ?1 AND status = 'pending'
```

## Call Stack

Delete a playlist (`DELETE /api/playlists/{id}`):
```
delete_playlist handler (id)
  → PlaylistDeleter::delete(id)
    → find_playlist(&id)
    → delete_playlist_videos(&id)
        for each PlaylistVideo:
          video_repository.delete(&video_id)
          … transcript / summary / categories / verdicts …
          task_repository.delete_pending_for_video(&video_id)   // new
        playlist_video_repository.delete_all_for_playlist(&id)
    → remove_pending_reconciles(&id)
    → delete_playlist(&id)
    → publish_playlist_deleted(&id, &playlist)
```

Delete a channel (`DELETE /api/channels/{id}`): the same through `ChannelDeleter::delete` → `delete_channel_videos`, `task_repository.delete_pending_for_video(&channel_video.video_id)` per video.

## Test Plan

1. Behaviour (HTTP handler tests, real SQLite)
   1. `it_should_remove_the_pending_tasks_of_a_deleted_playlists_videos`: a playlist video with pending `download_video` (run_at in the future), `fetch_thumbnail` and `summarize_video`; after delete, `list_non_completed` holds none of them.
   2. `it_should_keep_the_pending_tasks_of_another_playlists_videos`: deleting playlist A leaves the pending `download_video` of playlist B's video in place.
   3. `it_should_remove_the_pending_tasks_of_a_deleted_channels_videos`: as 1.1 for a channel.
   4. `it_should_keep_the_pending_tasks_of_another_channels_videos`: as 1.2 for a channel.
2. Infrastructure (`SqliteTaskRepository`)
   1. `it_should_delete_every_pending_task_of_a_video`: pending `download_video` and `fetch_thumbnail` of the video are gone.
   2. `it_should_not_delete_a_running_task_of_a_video`: a claimed `download_video` of the video is still listed as non-completed.
   3. `it_should_not_delete_the_pending_tasks_of_another_video`: another video's pending `download_video` and a `reconcile_playlist` are kept.
