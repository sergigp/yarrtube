# Design

The home card tints and the Categories field (in `CategoryPicker`) are frontend-only and already sit uncommitted in the working tree. So does a first category player, which fetched each selected video's source list from the frontend. This design replaces that fetch: the category videos endpoint returns full videos, so the category list alone feeds both the queue and the player.

## Files

Backend:
- `src/domain/video/category_video_view.rs`: new. Domain view of a category video, shaped like `VideoView` (video + metadata) plus `VideoSource` + `category_ids`. It sits next to `home_video_view.rs`.
- `src/domain/video/mod.rs`: registers and re-exports `CategoryVideoView`.
- `src/domain/services/video_searcher.rs`: `list_for_category` returns `Vec<CategoryVideoView>`, looking up each video's metadata through the existing `view`.
- `src/application/http/videos/dto.rs`: new flat `CategoryVideoResponse` (every `VideoResponse` field + `source` + `category_ids`; fields filled from the existing `VideoResponse` mapping, with no `#[serde(flatten)]` of one DTO into another). The `VideoSource` → `HomeVideoSourceResponse` mapping is pulled out of `HomeVideoResponse::from` into its own `From` impl, so both DTOs share it.
- `src/application/http/videos/mod.rs`: `list_videos_for_category` returns `Vec<CategoryVideoResponse>`. The existing category tests move to the new shape, and new tests are added.

Frontend (`web/src/`):
- `api/types.ts`: `CategoryVideo` type, mirroring the DTO.
- `api/client.ts`, `api/queries.ts`: `listCategoryVideos` / `useCategoryVideos` return `CategoryVideo[]`. The `useSourceVideos` hook from the earlier frontend-only version is removed.
- `test/helpers.tsx`: `aCategoryVideo` fixture builder.
- `components/CategoryPlayer.tsx`: new. Player + `VideoDetail` + queue, all fed from the category's videos.
- `components/CategoryDetail.tsx`: grid when there is no `?video=`, `CategoryPlayer` when there is; cards link to `/categories/:id?video=<id>`.
- `components/VideoGrid.tsx`: optional `videoPathOf`, so a card can link into the category.
- `components/VideoListPane.tsx`: generic over `T extends Video`, with `basePathOf(video)` replacing `basePath`, since a category's videos come from different source folders.
- `components/ChannelDetail.tsx`, `components/PlaylistDetail.tsx`: pass `basePathOf`.
- `hooks/useVideoSelection.ts`: generic over `T extends { id: string }`.
- `components/CategoryPicker.tsx`, `lib/categoryColors.ts`: picker chips and the 50% card tint (already done).
- Tests: `CategoryDetail.test.tsx`, `CategoryPicker.test.tsx`, `lib/categoryColors.test.ts`, `Home.test.tsx`, and the Add/Edit Channel/Playlist dialog tests.
- `e2e-tests/tests/categories.spec.js`: the picker step uses the chips.

## Types & Signatures

```rust
// src/domain/video/category_video_view.rs
/// A downloaded video of a category: as listed for its source, with the
/// source it is listed from and the categories having it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryVideoView {
    pub video: Video,
    pub metadata: Option<VideoMetadata>,
    pub source: VideoSource,
    /// Categories having the YouTube video, in category-name order.
    pub category_ids: Vec<CategoryId>,
}

// src/domain/services/video_searcher.rs (VideoSearcherApi)
fn list_for_category(&self, id: &CategoryId) -> Result<Vec<CategoryVideoView>, ListVideosError>;

// src/application/http/videos/dto.rs
#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryVideoResponse {
    pub id: String,
    pub title: String,
    pub status: String,
    pub quality: Option<String>,
    pub filename: Option<String>,
    pub thumbnail_filename: Option<String>,
    pub duration_seconds: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub watched: bool,
    pub position_seconds: i64,
    pub synced_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub channel_name: Option<String>,
    pub subtitle_language: Option<String>,
    pub source: HomeVideoSourceResponse,
    pub category_ids: Vec<String>,
}
impl From<CategoryVideoView> for CategoryVideoResponse;
impl From<VideoSource> for HomeVideoSourceResponse;

// src/application/http/videos/mod.rs
pub async fn list_videos_for_category(
    State(video_searcher): State<VideoSearcher>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CategoryVideoResponse>>, ApiError>;
```

```ts
// web/src/api/types.ts
/** An entry of `GET /api/categories/:id/videos`: a source's video plus where it's listed from. */
export interface CategoryVideo extends Video {
  source: HomeVideoSource
  category_ids: string[]
}
// CategoryVideo is assignable to HomeVideo, so VideoGrid takes it unchanged.

// web/src/components/CategoryPlayer.tsx
export function CategoryPlayer(props: { videos: CategoryVideo[] | undefined; error: Error | null }): JSX.Element

// web/src/components/VideoListPane.tsx
interface VideoListPaneProps<T extends Video> { noun: string; basePathOf: (video: T) => string; videos: T[] | undefined; /* … */ }

// web/src/components/VideoGrid.tsx
videoPathOf?: (video: HomeVideo) => string

// web/src/hooks/useVideoSelection.ts
export function useVideoSelection<T extends { id: string }>(videos: T[] | undefined): VideoSelection<T>
```

## Call Stack

List category videos:
```
GET /api/categories/{id}/videos
  list_videos_for_category(State(video_searcher), Path(id))
    CategoryId::new(id)
    run_blocking -> video_searcher.list_for_category(&id)
      find_category(id)
      category_video_collector.downloaded_videos(&category) -> Vec<SourcedVideo>
      once_per_youtube_video(..); sort by created_at desc
      category_ids_by_video()
      for each SourcedVideo { video, source }:
        video_metadata_repository.find(&video.id) -> metadata
        CategoryVideoView { video, metadata, source, category_ids: by_video[youtube_id] }
    -> Vec<CategoryVideoResponse::from>
```

Play in a category (frontend):
```
CategoryDetail (/categories/:id[?video=])
  useCategoryVideos(id) -> CategoryVideo[]
  no ?video -> VideoGrid(videoPathOf = v => /categories/:id?video=v.id)
  ?video    -> CategoryPlayer(videos)
                 useVideoSelection(videos) -> selected (deep link autoplays)
                 VideoPlayer(basePath = selected.source.path, source = selected.source, video = selected)
                 useWatchProgress(element, selected)  -> POST /api/videos/{id}/progress
                 VideoDetail(basePath = selected.source.path, channel = channels[selected.source.id] if channel)
                 VideoListPane(videos, basePathOf = v => v.source.path)
```

## Test Plan

1. Behaviour (HTTP handler, `src/application/http/videos/mod.rs`):
   - Existing `list_for_category` tests: assertions move from `HomeVideoResponse` to `CategoryVideoResponse` through a `category_video_response(..)` helper, with the same behaviour (order, dedupe from the channel, only downloaded, excluded-from-home playlist, empty, 404, categories).
   - `it_should_list_category_videos_with_their_playback_fields`: the entry has status `DOWNLOADED`, quality, file name, sync time and subtitle language `en`.
   - `it_should_list_category_videos_with_their_metadata`: the entry has publish time, description and channel name from the generated metadata.
   - `it_should_list_category_videos_without_metadata_as_absent`: publish time, description and channel name are `None`; the other fields are still present.
2. Frontend (Vitest):
   - `CategoryDetail`: "opens a card in the category, playing it with the category as the queue". The deep link autoplays from the playlist's path; selecting a channel video in the queue plays it from the channel path and shows the channel's avatar link; only `GET /api/categories/chess/videos` and the library lists are routed, with no source video lists.
   - `CategoryPicker`, `categoryColors`, `Home`, dialog tests: already updated.
3. Infrastructure: none. No adapter changes; metadata comes from the existing `video_metadata_repository`.
