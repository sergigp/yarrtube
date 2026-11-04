# Design

## Files

- `src/domain/video/video_view.rs`: `VideoView` carries the summary.
- `src/domain/video/category_video_view.rs`: `CategoryVideoView` carries the summary.
- `src/domain/services/video_searcher.rs`: reads summaries and decides visibility, as it already owns the listing read path.
- `src/infrastructure/repositories/sqlite_video_summary_repository.rs`: drop `#[allow(dead_code)]` and its comment from `find`, which now has a reader.
- `src/serve.rs`: pass `video_summary_repository` to `VideoSearcher::new`.
- `src/application/http/videos/dto.rs`: `summary` on `VideoResponse` and `CategoryVideoResponse`.
- `src/application/http/videos/mod.rs`: new argument at the 74 `VideoSearcher::new` calls, `summary` in the response helpers, the tests below.
- `web/src/api/types.ts`: `summary` on `Video`, mirroring the DTO.
- `web/src/test/helpers.tsx`: `summary: null` in `aVideo`.
- `web/src/components/VideoDetail.tsx`: render the summary.
- `web/src/components/VideoDetail.test.tsx`: summary tests.
- `web/src/api/queries.ts`: `useSavePreferences` refetches video lists.
- `web/src/components/AiSettingsView.test.tsx`: refetch test.
- `web/src/components/SummariesField.tsx`: new hint.
- `web/src/components/AddChannelDialog.test.tsx`, `AddPlaylistDialog.test.tsx`: new hint text.
- `doc/AI.md`, `doc/ARCHITECTURE.md`: summaries are shown, and when.

## Types & Signatures

```rust
pub struct VideoView {
    pub video: Video,
    pub metadata: Option<VideoMetadata>,
    pub summary: Option<VideoSummary>,
}

pub struct CategoryVideoView {
    // ...
    pub summary: Option<VideoSummary>,
}

pub struct VideoResponse {
    // ...
    pub summary: Option<String>,
}

pub struct CategoryVideoResponse {
    // ...
    pub summary: Option<String>,
}

impl VideoSearcher {
    pub fn new(/* ...existing..., */ video_summary_repository: Arc<dyn VideoSummaryRepository>) -> Self;

    /// `video` with its metadata, and its summary when `summaries_shown`.
    fn view(&self, video: Video, summaries_shown: bool) -> anyhow::Result<VideoView>;

    fn category_view(
        &self,
        sourced: SourcedVideo,
        by_video: &HashMap<VideoId, Vec<CategoryId>>,
        summaries_available: bool,
    ) -> anyhow::Result<CategoryVideoView>;

    /// Whether `source` opted in; false when it is no longer stored.
    fn source_summaries_enabled(&self, source: &VideoSource) -> anyhow::Result<bool>;
}
```

```ts
interface Video {
  // ...
  /** The AI summary, null when the video has none or it is hidden. */
  summary: string | null
}
```

The row's presence decides "we have one", so `summary_state` isn't consulted: an extra check would need a state the database can't otherwise reach to test it.

Visibility is computed per listing, not stored, so toggling needs no writes and restores summaries at once.

## Call Stack

Playlist or channel listing:

```
GET videos handler
  -> VideoSearcher::list_for_playlist(id)
       playlist_repository.find(id)                 // kept, was only checked
       find_preferences().summaries_available()
         && playlist.summaries_enabled              // = summaries_shown
       per video: view(video, summaries_shown)
         -> video_metadata_repository.find(&video.id)
         -> summaries_shown: video_summary_repository.find(&video.id)
  -> VideoResponse::from(view)                      // summary.text.as_str()
```

Category listing:

```
GET category videos handler
  -> VideoSearcher::list_for_category(id)
       find_preferences().summaries_available()     // read once
       per sourced: category_view(sourced, by_video, summaries_available)
         -> summaries_available && source_summaries_enabled(&sourced.source)
         -> view(sourced.video, summaries_shown)
  -> CategoryVideoResponse::from(view)              // via VideoResponse
```

Detail pane:

```
VideoDetail({ basePath, video, channel, playbackRate, onPlaybackRateChange })
  -> title row, MetaLine
  -> video.summary: <section aria-label="AI summary"> label + whitespace-pre-line text
  -> status Badge, storage location
```

Saving preferences:

```
useSavePreferences()(patch)
  -> patchPreferences(patch)
  -> invalidateQueries(recentVideos, channels, playlists, categories; refetchType 'all')
```

`refetchType: 'all'` because the AI settings page unmounts the lists: without it, going back paints the cached summary before the refetch.

## Test Plan

1. Behaviour tests

   Rust, in `src/application/http/videos/mod.rs`, real SQLite as its other tests:

   - `it_should_include_the_summary_when_listing_playlist_videos`: AI and summaries on, playlist opted in, stored summary: the entry has its text.
   - `it_should_hide_the_summary_when_ai_is_off`: same with AI off: `summary` is `None`, row still stored.
   - `it_should_hide_the_summary_when_summaries_are_off`: AI on, summaries off: `None`.
   - `it_should_hide_the_summary_of_a_playlist_with_summaries_off`: settings on, playlist opted out: `None`.
   - `it_should_include_the_summary_when_listing_channel_videos`: channel opted in: the entry has its text.
   - `it_should_hide_the_summary_of_a_channel_with_summaries_off`: channel opted out: `None`.
   - `it_should_report_no_summary_for_a_video_without_one`: everything on, no stored summary: `None`.
   - `it_should_include_the_summary_when_listing_category_videos`: member channel opted in: the entry has its text.
   - `it_should_hide_the_summary_of_a_category_video_when_ai_is_off`: `None`.
   - `it_should_hide_the_summary_of_a_category_video_whose_source_opted_out`: playlist member opted out: `None`.
   - `it_should_follow_the_channel_copy_of_a_category_video`: channel opted out, playlist opted in, both copies summarized: returned from the channel with `None`.

   Web:

   - VideoDetail `shows the AI summary`: region "AI summary" holds the exact text.
   - VideoDetail `shows no AI summary label when the video has none`.
   - AiSettingsView `refetches video lists after saving`: a cached channel video list, off screen, is requested again.
   - AddChannelDialog and AddPlaylistDialog: hint assertions updated to the new text.

2. Infrastructure tests

   None: `SqliteVideoSummaryRepository::find` already has its tests.

## Decisions

**Stale summaries can flash.** Lists are refetched after a save, not before it. So returning before a refetch lands, or after one fails, can briefly paint a cached summary that was just turned off, and turning off a source's toggle can do the same for off-screen lists. Accepted: it heals itself on the next fetch and the API never sends a hidden summary.
