# Design

## Files

- `src/domain/video/video.rs`: `with_watch_state_of`, which carries another copy's `watched_at`, `playback_position` and `last_played_at` over.
- `src/domain/services/video_downloader.rs`: new private `save`, which re-reads the video and keeps its current watch state before `update`. `record_downloaded`, `exclude` and `mark_errored` use it instead of `video_repository.update`.
- `src/application/tasks/download_video_task.rs`: behaviour tests (the existing `FakeVideoDownloaderRepository::with_on_download` hook records the watch state mid-download).

## Types & Signatures

```rust
// domain/video/video.rs
impl Video {
    /// This video with `current`'s watch state: `watched_at`,
    /// `playback_position` and `last_played_at`.
    pub fn with_watch_state_of(self, current: &Video) -> Self;
}

// domain/services/video_downloader.rs (private impl)
impl VideoDownloader {
    /// Saves `video` with the watch state stored now, not the one read when
    /// the download started; a video gone meanwhile is saved as given.
    fn save(&self, video: Video) -> anyhow::Result<Video>;
}
```

Not a transaction: a watch-state change between the re-read and the `update` (sub-millisecond) is still lost. This is accepted; narrow repository writes were rejected.

## Call Stack

Download (the read and the `start_download` save are unchanged):
```
VideoDownloader::download(video_id, quality, output_dir, is_last_attempt)
  video_repository.find(&video_id) -> started = video.start_download(now); video_repository.update(&started)
  run_download(&started, ..)                        // minutes; watch state may change meanwhile
  Succeeded -> record_unless_deleted -> place_download
    record_downloaded(started, ..)
      save(started.mark_downloaded(..).with_episode_number(..))
  Failed (permanent) -> exclude(started, reason)
      save(started.mark_excluded(now))
  Failed / Err / placement error -> mark_errored(video, is_last_attempt)
      save(video.mark_errored(now) | video.mark_errored_retrying(now))

save(video)
  video_repository.find(&video.id)?
    Some(current) -> video.with_watch_state_of(&current)
    None          -> video
  video_repository.update(&saved) -> Ok(saved)
```

## Test Plan

Behaviour tests (`download_video_task.rs`; each test's `with_on_download` hook stores the in-progress video with `watched_at`, a playback position and `last_played_at` set):

1. `it_should_keep_the_watch_state_recorded_while_downloading`: the download succeeds, and the stored video is downloaded with the hook's watch state.
2. `it_should_keep_the_watch_state_recorded_while_a_failed_download_ran`: the download fails with retries left, and the stored video is errored-retrying with the hook's watch state.
3. `it_should_keep_the_watch_state_recorded_while_downloading_an_excluded_video`: the download fails for a permanent reason, and the stored video is excluded with the hook's watch state.
4. `it_should_keep_the_watch_state_recorded_while_downloading_if_placing_fails`: the move into place fails on the last attempt, and the stored video is errored, keeps its episode number and has the hook's watch state.

Infrastructure tests: none (no adapter changes).
