# Design

## Files

- `src/domain/video/home_category_section.rs` (new) — domain read model for one "More about" section, next to `HomeVideos`.
- `src/domain/video/home_videos.rs` — `HomeVideos` gains `categories`.
- `src/domain/video/home_limits.rs` — `HomeLimits` gains the category section limits.
- `src/domain/video/mod.rs` — exports `HomeCategorySection`.
- `src/domain/services/video_searcher.rs` — `list_home` picks the category sections between continue watching and quick watches.
- `src/application/http/videos/dto.rs` — `HomeCategorySectionResponse`, `HomeResponse.categories`.
- `src/application/http/videos/mod.rs` — `HOME_LIMITS` values; behaviour tests.
- `web/src/api/types.ts` — `HomeCategorySection`, `HomeVideos.categories` mirroring the DTO.
- `web/src/test/helpers.tsx` — `aHomeCategorySection` builder.
- `web/src/components/Home.tsx` — renders the category sections with a linked heading and "See all".
- `web/src/components/Home.test.tsx` — component tests.

## Types & Signatures

```rust
// src/domain/video/home_category_section.rs
/// A "More about <name>" home section: a category and its pending videos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeCategorySection {
    pub id: CategoryId,
    pub name: CategoryName,
    pub color: CategoryColor,
    pub videos: Vec<HomeVideoView>,
}

// src/domain/video/home_videos.rs
pub struct HomeVideos {
    pub continue_watching: Vec<HomeVideoView>,
    pub categories: Vec<HomeCategorySection>,
    pub quick_watches: Vec<HomeVideoView>,
    pub latest: Vec<HomeVideoView>,
}

// src/domain/video/home_limits.rs
pub struct HomeLimits {
    pub continue_watching: usize,
    pub category_sections: usize, // 3
    pub category_videos: usize,   // 6
    pub quick_watches: usize,
    pub latest: usize,
}

// src/domain/services/video_searcher.rs (private, impl VideoSearcher)
/// Category sections of `videos`, ranked by pending count (most first, ties
/// by name ignoring case), each without videos shown in earlier sections,
/// empty ones skipped, up to `limits.category_sections`.
fn category_sections_in(
    videos: &[SourcedVideo],
    categories: &[Category],
    by_video: &HashMap<VideoId, Vec<CategoryId>>,
    limits: HomeLimits,
) -> Vec<(Category, Vec<SourcedVideo>)>;

/// Downloaded, unwatched videos of `videos` in `category` per `by_video`,
/// once per YouTube video (channel copy first), newest sync first. Untruncated.
fn pending_in(
    videos: &[SourcedVideo],
    category: &CategoryId,
    by_video: &HashMap<VideoId, Vec<CategoryId>>,
) -> Vec<SourcedVideo>;

// src/application/http/videos/dto.rs
#[derive(Debug, Serialize, PartialEq)]
pub struct HomeCategorySectionResponse {
    pub id: String,
    pub name: String,
    pub color: String,
    pub videos: Vec<HomeVideoResponse>,
}
impl From<HomeCategorySection> for HomeCategorySectionResponse;

pub struct HomeResponse {
    pub continue_watching: Vec<HomeVideoResponse>,
    pub categories: Vec<HomeCategorySectionResponse>,
    pub quick_watches: Vec<HomeVideoResponse>,
    pub latest: Vec<HomeVideoResponse>,
}
```

```ts
// web/src/api/types.ts
export interface HomeCategorySection {
  id: string
  name: string
  color: string
  videos: HomeVideo[]
}
export interface HomeVideos {
  continue_watching: HomeVideo[]
  categories: HomeCategorySection[]
  quick_watches: HomeVideo[]
  latest: HomeVideo[]
}
```

Ranking counts only the home pool (excluded playlists left out) minus continue watching, so it can differ from the sidebar's unwatched badge — intended.

## Call Stack

```
GET /api/videos/home
  list_home_videos(State<VideoSearcher>)
    VideoSearcher::list_home(HOME_LIMITS)
      downloaded_across_sources()                              -> pool
      continue_watching_in(&pool, now, 6)                      -> cw
      not_shown_in(&pool, &cw)                                 -> after_cw
      category_repository.list()                               -> categories
      category_ids_by_video(&categories)                       -> by_video
      category_sections_in(&after_cw, &categories, &by_video, limits)
        for each category: pending_in(&after_cw, &id, &by_video) -> count
        sort by (count desc, name lowercase asc)
        fold: pending_in(&remaining, ...) -> truncate(6) -> skip if empty
              remaining = not_shown_in(&remaining, &section)
              stop at 3 sections
      not_shown_in(&after_cw, all section videos)              -> after_categories
      quick_watches_in(&after_categories, 6)                   -> qw
      latest_in(not_shown_in(&after_categories, &qw), 18)      -> latest
      with_categories(views(..), &by_video) for every list
    HomeResponse::from(HomeVideos)
```

`category_ids_by_video` takes the already listed categories instead of listing them again.

```
Home.tsx
  useRecentVideos() -> home
  HomeSection "Continue watching"
  home.categories.map -> HomeSection title "More about {name}", href /categories/{id}, "See all" link, hideWhenEmpty
  HomeSection "Quick watches"
  HomeSection "Latest videos"
```

## Test Plan

### 1. Behaviour — HTTP handler (`src/application/http/videos/mod.rs`)

1. `it_should_list_a_category_section_with_its_pending_videos_on_home` — one category with 2 unwatched videos → one section with id, name, color, both videos newest sync first.
2. `it_should_list_no_category_sections_if_nothing_pending_on_home` — every category video watched → `categories` empty.
3. `it_should_cap_a_category_section_at_6_on_home` — 10 pending → 6 most recent.
4. `it_should_list_a_category_section_video_once_from_the_channel_on_home` — video in channel and playlist of the category → once, channel source.
5. `it_should_not_repeat_a_continue_watching_video_in_category_sections_on_home` — in-progress video of the category → only under continue watching.
6. `it_should_not_repeat_a_category_section_video_in_quick_watches_on_home` — short pending video of a category → in section, not in quick watches.
7. `it_should_not_repeat_a_category_section_video_in_latest_on_home` — pending video shown in section → not in latest.
8. `it_should_rank_category_sections_by_pending_videos_on_home` — 4 / 9 / 1 pending → Science, Chess, Streams.
9. `it_should_count_every_pending_video_when_ranking_category_sections_on_home` — 8 vs 7 pending (both show 6) → 8 first.
10. `it_should_not_count_continue_watching_videos_when_ranking_category_sections_on_home` — A 3 pending + 2 in progress, B 4 pending → B first.
11. `it_should_break_category_section_ties_by_name_on_home` — "chess" and "Astronomy" tied → Astronomy first.
12. `it_should_cap_category_sections_at_3_on_home` — 5 categories with pending → top 3.
13. `it_should_list_a_video_in_two_categories_in_the_first_section_only_on_home` — shared pending video → only in higher ranked section.
14. `it_should_skip_a_category_emptied_by_earlier_sections_on_home` — second's videos all in first, fourth has pending → sections 1st, 3rd, 4th.
15. `it_should_leave_videos_of_a_playlist_excluded_from_home_out_of_category_sections_on_home` — category member excluded from home, only source → no section.
16. `it_should_show_a_category_video_through_a_source_shown_on_home` — excluded playlist in category + uncategorised channel tracking same video → in section with channel source.

### 2. Behaviour — Web (`web/src/components/Home.test.tsx`)

17. `shows a "More about" section per category between continue watching and quick watches` — headings order CW, More about Science, More about Chess, Quick watches.
18. `navigates to the category from the section heading` — click heading → `/categories/chess`.
19. `navigates to the category from "See all"` — click "See all" → `/categories/chess`.
20. `shows no "More about" section when the response has none`.
21. `hides an empty "Latest videos" when other sections have videos` — only category sections have videos → no "Latest videos" heading.
22. `shows "No videos synced yet." when every section is empty` — all sections empty → "Latest videos" with the message.

### 3. Infrastructure

None — no adapter changes.
