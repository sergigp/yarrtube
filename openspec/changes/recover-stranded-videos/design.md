## Files

- `src/domain/video/video.rs` adds `is_download_settled`, the status predicate shared by the stranded check and the downloader guard.
- `src/domain/task/scheduled_task.rs` adds `download_video_id`, which reads the video a queued download task targets.
- `src/domain/services/playlist_video_reconciler.rs` adds the stranded-video pass in `reconcile_filesystem`, after the Errored recovery loop.
- `src/domain/services/channel_video_reconciler.rs` adds the same pass for channels.
- `src/domain/services/video_downloader.rs` makes `download` skip a video that is already settled.
- `src/application/tasks/reconcile_playlist_task.rs`, `reconcile_channel_task.rs` and `download_video_task.rs` hold the behaviour tests.

## Types & Signatures

```rust
// video.rs
impl Video {
    /// Downloaded, Excluded or Errored: nothing left for a queued download to do.
    pub fn is_download_settled(&self) -> bool;
}

// scheduled_task.rs
impl ScheduledTask {
    /// The video id of a `download_video` task; None for any other type.
    pub fn download_video_id(&self) -> Option<String>;
}

// playlist_video_reconciler.rs / channel_video_reconciler.rs (private)
fn video_ids_with_download_in_flight(&self) -> anyhow::Result<HashSet<String>>; // via task_repository.list_non_completed()
```

No trait or port changes.

## Call Stack

Reconcile pass (playlist; channel is identical with `channel.quality`):

```
PlaylistVideoReconciler::reconcile_filesystem(playlist, changes)
├─ in_flight = video_ids_with_download_in_flight()        // read BEFORE stored videos (see note)
│  └─ task_repository.list_non_completed() → filter_map(ScheduledTask::download_video_id)
├─ stored_videos = playlist_video_repository.list_for_playlist + video_repository.find
├─ … existing downloaded-file checks, Errored recovery (fills skip_thumbnail_ids)
├─ for v in stored_videos where !v.is_download_settled()
│        && !in_flight.contains(v.id) && !skip_thumbnail_ids.contains(&v.id):
│     warn!(playlist_id, video_id, status, "stranded video found during reconcile, rescheduling its download")
│     task_repository.schedule(Task::DownloadVideo { video_id, quality, output_dir }, now)   // dedupe still guards
└─ thumbnail_fetcher.schedule_missing(...)                  // unchanged
```

Download task:

```
DownloadVideoTask::handle(payload, is_last_attempt)
└─ VideoDownloader::download(video_id, quality, output_dir, is_last_attempt)
   ├─ video_repository.find(video_id) → None → Ok(())          // existing
   ├─ video.is_download_settled() → debug!("video already settled, skipping download"); Ok(())   // new
   └─ start_download … (unchanged)
```

Read order: the in-flight task set is read before the videos. A task that is pending or running when the set is read keeps its video out of the stranded set. A task created after the read is either finished by the time the videos are read (the video is then settled) or rejected by schedule-time dedupe. The one remaining race, where a task is created and finishes between the video read and `schedule`, is handled by the settled guard in the downloader.

`skip_thumbnail_ids` already holds the ids this pass added or reset for redownload, which are exactly the ids the stranded check must skip, so the pass reuses that set.

## Test Plan

1. Behaviour (`reconcile_playlist_task`):
   - `it_should_reschedule_the_download_of_an_errored_retrying_video_with_no_download_task`: a stored Errored Retrying video with no tasks. Asserts `Ok(())`, `list_non_completed()` includes a `download_video` task for it, and the video is unchanged.
   - `it_should_reschedule_the_download_of_a_pending_video_with_no_download_task`: same setup with a Pending video.
   - `it_should_reschedule_the_download_of_an_in_progress_video_with_no_download_task`: same setup with an In Progress video.
   - `it_should_not_reschedule_a_non_terminal_video_whose_download_is_queued`: an Errored Retrying video with a pending `download_video` task. Asserts only that one download task exists.
   - `it_should_not_reschedule_a_downloaded_video`: a Downloaded video with its file on disk and no tasks. Asserts no download task is scheduled.
2. Behaviour (`reconcile_channel_task`):
   - `it_should_reschedule_the_download_of_an_errored_retrying_video_with_no_download_task`
   - `it_should_not_reschedule_a_non_terminal_video_whose_download_is_queued`
3. Behaviour (`download_video_task`):
   - `it_should_skip_the_download_of_an_already_downloaded_video`: Asserts `Ok(())`, the video is unchanged, and the fake downloader recorded no download.
   - `it_should_skip_the_download_of_an_excluded_video`
   - `it_should_skip_the_download_of_an_errored_video`
