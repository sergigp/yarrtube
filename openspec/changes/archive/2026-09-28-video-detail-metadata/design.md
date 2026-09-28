## Files

- `src/domain/video/video_view.rs`: new read model, a video paired with its generated metadata (if any), next to the other `*_view` read models.
- `src/domain/video/mod.rs`: export `VideoView`.
- `src/domain/services/video_searcher.rs`: takes `VideoMetadataRepository`. `list` and `list_for_channel` return `Vec<VideoView>`.
- `src/application/http/videos/dto.rs`: `VideoResponse` gains `published_at`, `description` and `channel_name`, and is built `From<VideoView>`.
- `src/application/http/videos/mod.rs`: handlers map `VideoView`; the tests construct the searcher with the metadata repository.
- `src/serve.rs`: pass `infrastructure.video_metadata_repository` to `VideoSearcher::new`.
- `web/src/components/VideoDetail.jsx`: meta line, description clamp with "Show more", links in the description, and the status badge only when not downloaded. The quality badge is removed.
- `smoke-tests/helpers/video.js`: `waitForVideoStatus` no longer relies on a "Downloaded" badge. For `DOWNLOADED` it waits for the "Synced" text on the meta line.

## Types & Signatures

```rust
// src/domain/video/video_view.rs
/// A video as listed, with its generated metadata when it has any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoView {
    pub video: Video,
    pub metadata: Option<VideoMetadata>,
}
```

```rust
// src/domain/services/video_searcher.rs
impl VideoSearcher {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    ) -> Self;

    fn view(&self, video: Video) -> anyhow::Result<VideoView>;
}

pub trait VideoSearcherApi: Send + Sync {
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<VideoView>, ListVideosError>;
    fn list_for_channel(&self, channel_id: &ChannelHandle) -> Result<Vec<VideoView>, ListVideosError>;
    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>; // unchanged
}
```

```rust
// src/application/http/videos/dto.rs
pub struct VideoResponse {
    // ...existing fields unchanged...
    pub published_at: Option<DateTime<Utc>>, // metadata.published_at
    pub description: Option<String>,         // metadata.plot (already truncated to 500 chars)
    pub channel_name: Option<String>,        // metadata.studio
}

impl From<VideoView> for VideoResponse;
```

```jsx
// web/src/components/VideoDetail.jsx (props unchanged)
export function VideoDetail({ basePath, video, channel })
// channel absent => playlist view => meta line leads with video.channel_name
```

## Call Stack

**List playlist videos**: `GET /playlists/{id}/videos`
1. `list_videos_for_playlist(State<VideoSearcher>, Path<String>)`
2. → `VideoSearcher::list(&PlaylistId)`
   - → `PlaylistRepository::find(&PlaylistId)`
   - → `PlaylistVideoRepository::list_for_playlist(&PlaylistId)`
   - → for each `PlaylistVideo`:
     - `VideoRepository::find(&VideoRecordId)`
     - → `VideoSearcher::view(Video)`
     - → `VideoMetadataRepository::find(&VideoRecordId)`
3. → `VideoResponse::from(VideoView)` for each

**List channel videos**: `GET /channels/{handle}/videos`
Same as above, via `VideoSearcher::list_for_channel(&ChannelHandle)`, `ChannelRepository::find` and `ChannelVideoRepository::list_for_channel`.

**Render detail pane**
1. `PlaylistDetail` / `ChannelDetail`
2. → `VideoDetail({ basePath, video, channel })`
3. → meta line from:
   - `channel ? null : video.channel_name`
   - `video.published_at` (date only)
   - `video.synced_at` (`formatRelativeTime`, with `formatDateTime` as the title)

## Test Plan

**Behaviour tests** (`src/application/http/videos/mod.rs`)
1. `it_should_include_the_metadata_when_listing_playlist_videos`: a downloaded playlist video with a saved `video_metadata` row. The response has `published_at`, `description` (the plot) and `channel_name` (the studio) set.
2. `it_should_include_the_metadata_when_listing_channel_videos`: the same, through the channel listing.
3. `it_should_report_absent_metadata_when_listing_a_video_without_it`: a pending playlist video with no metadata row. The response has `published_at`, `description` and `channel_name` all `None`, and every other field is unchanged.

**Infrastructure tests**
None. The change reuses `SqliteVideoMetadataRepository::find`, which is already covered.
