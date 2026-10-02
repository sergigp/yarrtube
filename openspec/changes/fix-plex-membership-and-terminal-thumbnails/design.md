## Files

- `src/infrastructure/repositories/plex_collection_repository.rs`: `add_items`/`remove_item` target `/library/collections/{key}/items[/{ratingKey}]`. The `/library/metadata/...` form 404s. Mocks are updated to match.
- `src/domain/services/thumbnail_fetcher.rs`:
  - `schedule_missing` skips `Excluded`/`Errored` videos.
  - `fetch` returns early for those statuses.
  - The "no thumbnail" warn carries the tool's reason.
- `src/domain/video/video.rs`: `Video::is_thumbnail_fetchable`, the one predicate that both the scheduling and fetch sides use.
- `src/infrastructure/repositories/youtube_video_downloader_repository.rs`: the port's `fetch_thumbnail` returns `ThumbnailFetch` instead of `Option<FetchedThumbnail>`, so a clean failure carries its reason. The fake's `with_thumbnail_result(Option<FetchedThumbnail>)` builder keeps its signature and maps `None` to `Unavailable { reason: None }`.
- `src/infrastructure/shared/ytdlp.rs`:
  - `fetch_thumbnail` captures stderr (`Stdio::piped()`) and returns the warning-free reason.
  - `list_channel_videos` captures stderr and returns `Err` carrying the reason on a non-zero exit. Today it returns `Ok(vec![])`, which the reconciler reads as "channel has no videos" and evicts every stored video. That contradicts `channel-video-sync`'s "yt-dlp fails to list a channel's videos" scenario.
- `src/application/tasks/fetch_thumbnail_task.rs`, `reconcile_playlist_task.rs`, `reconcile_channel_task.rs`: behaviour tests only.

## Types & Signatures

```rust
// youtube_video_downloader_repository.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailFetch {
    Fetched(FetchedThumbnail),
    /// Clean yt-dlp failure, or success that printed no thumbnail.
    Unavailable { reason: Option<String> },
}

pub trait VideoDownloaderRepository {
    fn fetch_thumbnail(
        &self,
        video_url: &str,
        desired_filename: &str,
        video_id: &str,
        output_dir: &Path,
        existing_folder: Option<&str>,
    ) -> anyhow::Result<ThumbnailFetch>; // was Result<Option<FetchedThumbnail>>
}

// ytdlp.rs
pub fn fetch_thumbnail(/* unchanged params */) -> Result<ThumbnailFetch>;
pub fn list_channel_videos(ytdlp_path: &Path, channel_url: &str, limit: u32)
    -> Result<Vec<ChannelVideoEntry>>; // non-zero exit -> Err("yt-dlp failed to list channel videos for {url}: {reason}")

// video.rs
impl Video {
    /// False for Excluded (never fetchable) and Errored (recovery redownloads it).
    pub fn is_thumbnail_fetchable(&self) -> bool;
}

// plex_collection_repository.rs (paths only)
// add_items:   PUT    /library/collections/{collection_rating_key}/items?uri=…
// remove_item: DELETE /library/collections/{collection_rating_key}/items/{rating_key}
```

## Call Stack

Thumbnail recovery (per reconcile pass):
`ReconcilePlaylistTask::handle` / `ReconcileChannelTask::handle` → `*VideoReconciler::reconcile_filesystem` → `ThumbnailFetcher::schedule_missing(videos, skip_ids, output_dir)` → filter `thumbnail_filename.is_none() && !skip_ids.contains(id) && status != InProgress && video.is_thumbnail_fetchable()` → `TaskRepository::schedule(Task::FetchThumbnail { video_id, output_dir }, now)`

Thumbnail fetch:
`FetchThumbnailTask::handle(payload)` → `VideoRepository::find(video_id)` → `ThumbnailFetcher::fetch(&video, output_dir)` → return if `thumbnail_filename.is_some() || !video.is_thumbnail_fetchable()` → `VideoDownloaderRepository::fetch_thumbnail(url, filename, youtube_id, output_dir, existing_folder)` → `ytdlp::fetch_thumbnail` (stderr piped) →
- `Fetched` → `VideoRepository::update_thumbnail`
- `Unavailable { reason }` → `warn!(video_id, reason, "no thumbnail available for video")`

Channel listing:
`ChannelVideoReconciler::sync_channel_membership` → `YtDlpChannelVideosRepository::list_current_videos(id, limit)` → `ytdlp::list_channel_videos(path, url, limit)` (stderr piped) → non-zero exit → `Err(reason)` → `?` propagates before any membership change, and the task executor logs it with `task_id` and the error cause chain. This is existing behaviour, already covered by `it_should_keep_videos_if_listing_fails` via the fake.

Plex membership: unchanged call stack. `PlexCollectionReconciler::converge_members` → `add_items` / `remove_item` with the new paths.

## Test Plan

1. Behaviour tests (application layer):
   1. `fetch_thumbnail_task::it_should_not_fetch_a_thumbnail_for_an_excluded_video`: Excluded video with no thumbnail. The fake downloader records no `fetch_thumbnail` call and the stored video is unchanged.
   2. `fetch_thumbnail_task::it_should_not_fetch_a_thumbnail_for_an_errored_video`: same, for an Errored video.
   3. `reconcile_playlist_task::it_should_not_schedule_a_thumbnail_fetch_for_an_excluded_video`: Excluded video with no thumbnail. No `FetchThumbnail` task is scheduled; the only pending task is the next reconcile.
   4. `reconcile_playlist_task::it_should_not_schedule_a_thumbnail_fetch_for_an_errored_video_not_due_for_recovery`: Errored within cooldown. No `FetchThumbnail` task.
   5. `reconcile_channel_task::it_should_not_schedule_a_thumbnail_fetch_for_an_excluded_video`: channel counterpart of 1.3.
   6. `reconcile_channel_task::it_should_not_schedule_a_thumbnail_fetch_for_an_errored_video_not_due_for_recovery`: channel counterpart of 1.4.
   7. The existing `it_should_schedule_a_thumbnail_fetch_for_a_video_missing_one` tests (Pending/Downloaded) stay green.
2. Infrastructure tests:
   - `ytdlp.rs` (fake yt-dlp script):
     1. `it_should_return_unavailable_with_the_reason_on_a_clean_failed_thumbnail_fetch`: the script writes `ERROR: [youtube] x: Video unavailable` to stderr and exits 1. Returns `Unavailable { reason: Some("ERROR: [youtube] x: Video unavailable") }` and the folder is removed (folds into the existing `..._on_a_clean_failed_exit` test).
     2. `it_should_return_unavailable_without_a_reason_when_yt_dlp_prints_na`: adapts the existing NA test to the new type.
     3. `it_should_error_with_the_reason_when_listing_channel_videos_fails`: the script writes an error to stderr and exits 1. `Err` whose message contains the stderr reason.
     4. `it_should_list_channel_videos_from_the_printed_json_lines`: the happy path, if not already covered.
   - `plex_collection_repository.rs` (mockito): `it_should_add_items_to_a_collection` and `it_should_remove_an_item_from_a_collection` assert the `/library/collections/...` paths. Already updated in the working tree.
