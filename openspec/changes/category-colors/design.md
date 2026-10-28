# Design

## Files

- `migrations/0013_category_color.sql`: adds `categories.color` and backfills existing rows in palette order by `created_at`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers migration 0013.
- `src/domain/category/category_color.rs` (new): the `CategoryColor` value object (validation, lowercasing), the palette and the least-used default.
- `src/domain/category/mod.rs`: exports `CategoryColor`.
- `src/domain/category/category.rs`: adds a `color` field, `create` takes a color, adds `with_color`.
- `src/domain/category/category_view.rs`: `CategoryView` carries `color`.
- `src/domain/services/category_creator.rs`: an optional color, defaulting to the least-used palette color.
- `src/domain/services/category_updater.rs`: `rename` becomes `update` (name and/or color, saved once).
- `src/domain/services/category_view_searcher.rs`: passes `color` into `CategoryView`.
- `src/domain/video/home_video_view.rs`: adds `category_ids` and `with_category_ids`.
- `src/domain/services/video_searcher.rs`: fills `category_ids` for home and category listings.
- `src/infrastructure/repositories/sqlite_category_repository.rs`: reads and writes `color`.
- `src/application/http/categories/dto.rs`: `color` on requests and responses.
- `src/application/http/categories/mod.rs`: parses the color; the update handler accepts name and/or color.
- `src/application/http/videos/dto.rs`: `category_ids` on `HomeVideoResponse`.
- `web/src/api/types.ts`: `color` on `Category`/`CreatedCategory`, `category_ids` on `HomeVideo`.
- `web/src/api/client.ts`: `createCategory(name, color)`, `updateCategory(id, { name?, color? })` (replaces `renameCategory`).
- `web/src/test/helpers.tsx`: fixture builders get the new fields.
- `web/src/lib/categoryColors.ts` (+ test, new): the palette (names and hex), `defaultColor`, `pastelFromHue`, `hueOf`, `cardBackground`.
- `web/src/components/CategoryColorPicker.tsx` (+ test, new): swatch radio group plus the custom hue slider. Each radio input covers its swatch, invisible, so a click anywhere on it (and a click by role) chooses it.
- `web/src/components/AddCategoryDialog.tsx` (+ test): the picker, with the default color preselected.
- `web/src/components/CategoryDetail.tsx` (+ test): header tint, the "Change color" popover, an untinted grid.
- `web/src/components/DetailHeader.tsx`: optional `tint` background and a slot for the color control.
- `web/src/components/SidebarCategories.tsx` (+ test): the color dot.
- `web/src/components/VideoGrid.tsx`: a `categoryColors` prop; with it, every card is framed (padding, a thin grey border, 8px radius, the grid gap kept) and takes `cardBackground` as its background.
- `web/src/components/Home.tsx` (+ test): passes the categories' colors (`categoryColors`) to the grid.

## Types & Signatures

```rust
// domain/category/category_color.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryColor(String); // "#rrggbb", lowercase

impl CategoryColor {
    pub const PALETTE: [&'static str; 10] = [
        "#fff1bf", "#ffe0c7", "#ffd9d4", "#fadceb", "#e9def8",
        "#dbe3fb", "#d3ecf8", "#cff0ea", "#e4f0cc", "#eee5d7",
    ];
    pub fn new(color: impl Into<String>) -> Result<Self, ValidationError>;
    /// Palette color used by the fewest of `used`, earliest on a tie.
    pub fn least_used(used: &[CategoryColor]) -> Self;
    pub fn as_str(&self) -> &str;
}

// domain/category/category.rs
pub struct Category { pub id, pub name, pub color: CategoryColor, pub members, pub created_at }
impl Category {
    pub fn create(name: CategoryName, color: CategoryColor, now: DateTime<Utc>) -> Self;
    pub fn with_color(self, color: CategoryColor) -> Self;
}

// domain/category/category_view.rs
pub struct CategoryView { /* … */ pub color: CategoryColor }

// domain/services/category_creator.rs
fn create(&self, name: CategoryName, color: Option<CategoryColor>) -> Result<Category, CreateCategoryError>;

// domain/services/category_updater.rs
pub struct CategoryUpdate { pub name: Option<CategoryName>, pub color: Option<CategoryColor> }
fn update(&self, id: &CategoryId, update: CategoryUpdate) -> Result<(), UpdateCategoryError>; // replaces rename

// domain/video/home_video_view.rs
pub struct HomeVideoView { /* … */ pub category_ids: Vec<CategoryId> } // name order
impl HomeVideoView { pub fn with_category_ids(self, ids: Vec<CategoryId>) -> Self; }

// domain/services/video_searcher.rs (private)
/// YouTube video → ids of categories having it downloaded, in category-name order.
fn category_ids_by_video(&self) -> anyhow::Result<HashMap<VideoId, Vec<CategoryId>>>;
fn with_categories(&self, views: Vec<HomeVideoView>) -> Result<Vec<HomeVideoView>, ListVideosError>;

// application/http/categories/dto.rs
pub struct CreateCategoryRequest { pub name: Option<String>, pub color: Option<String> }
pub struct UpdateCategoryRequest { pub name: Option<String>, pub color: Option<String> }
pub struct CategoryResponse { /* … */ pub color: String }
pub struct CategoryListItemResponse { /* … */ pub color: String }

// application/http/videos/dto.rs
pub struct HomeVideoResponse { /* … */ pub category_ids: Vec<String> }
```

```ts
// web/src/lib/categoryColors.ts
export const PALETTE: readonly { name: string; hex: string }[] // Butter … Sand
export const CROWDED_CARD = '#f2f2f2'
export function defaultColor(used: string[]): string
export function pastelFromHue(hue: number): string   // oklch(0.94 0.04 hue) -> #rrggbb; chroma lowered until it fits sRGB, so the hue holds
export function hueOf(hex: string): number
export function cardBackground(colors: string[]): string | undefined
// 0 -> undefined, 1 -> hex, 2-3 -> linear-gradient(to bottom right, hard stops), 4+ -> CROWDED_CARD
// `to bottom right` (not a fixed angle) puts the 50% edge exactly corner to corner on any card shape

// web/src/components/CategoryColorPicker.tsx
export function CategoryColorPicker(props: { value: string; onChange: (hex: string) => void }): JSX.Element

// web/src/components/VideoGrid.tsx
export function VideoGrid(props: { videos: HomeVideo[]; categoryColors?: Map<string, string> }): JSX.Element
// categoryColors absent -> untinted (category view)
```

## Call Stack

Create: `POST /api/categories {name, color?}`
- `create_category` → `CategoryName::new(name)`, `color.map(CategoryColor::new)`
- → `CategoryCreator::create(name, color)` → `ensure_name_free` → `color.unwrap_or_else(|| CategoryColor::least_used(&existing colors))`
- → `Category::create(name, color, now)` → `CategoryRepository::insert` → 201 `CategoryResponse`

Update: `PATCH /api/categories/:id {name?, color?}`
- `update_category` → both absent → 400; parse each present field
- → `CategoryUpdater::update(&id, CategoryUpdate { name, color })` → `find_category` → `name.map(ensure_name_free)` → apply `with_name`/`with_color` → `repository.update` (one save) → 204

List: `GET /api/categories` → `CategoryViewSearcher::search_all` → `CategoryView { color, … }` → `CategoryListItemResponse { color }`

Home: `GET /api/videos/home`
- `list_home_videos` → `VideoSearcher::list_home(limits)` → sections as today
- → `with_categories(views)` → `category_ids_by_video()`:
  `category_repository.list()` (name order) → for each `category_video_collector.downloaded_videos(&category)` → insert `category.id` under each `youtube_id`, once
- → `view.with_category_ids(map[youtube_id] or [])` → `HomeVideoResponse { category_ids }`

Category videos: `GET /api/categories/:id/videos` → `VideoSearcher::list_for_category(id)` → `with_categories(views)` → same as home

Web home card: `Home` → `useCategories()` → `Map(id → color)` → `VideoGrid categoryColors` → per card `cardBackground(video.category_ids.slice(0, 4).flatMap(id → color))` → `style={{ background }}`

## Test Plan

1. Behaviour tests (HTTP handlers)
   - `categories::it_should_create_a_category_with_a_color`: 201 with the given color, lowercased.
   - `categories::it_should_create_a_category_with_the_least_used_palette_color`: no color, with Butter and Coral used → `#ffe0c7`.
   - `categories::it_should_create_the_first_category_with_the_first_palette_color`: no categories → `#fff1bf`.
   - `categories::it_should_fail_to_create_if_color_invalid`: `"#fff"` → 400, nothing stored.
   - `categories::it_should_list_categories_with_their_color`: the list item carries `color`.
   - `categories::it_should_recolor_a_category`: PATCH with color only → 204, name and members kept.
   - `categories::it_should_rename_and_recolor_a_category`: both fields → both applied.
   - `categories::it_should_fail_to_update_without_name_or_color`: 400, unchanged.
   - `categories::it_should_fail_to_update_if_color_invalid`: 400, unchanged.
   - `categories::it_should_not_recolor_if_name_taken`: 409, color unchanged.
   - `videos::it_should_list_a_home_video_with_its_category`: source in "Chess" → `category_ids` = [Chess].
   - `videos::it_should_list_a_home_video_without_categories`: → `[]`.
   - `videos::it_should_order_home_video_categories_by_name`: "Streams" and "chess" → [chess, Streams].
   - `videos::it_should_include_categories_reached_through_another_source`: channel copy shown, playlist (excluded from home) in "Highlights" → [Chess, Highlights].
   - `videos::it_should_list_a_category_reached_by_two_members_once`
   - `categories::it_should_list_category_videos_with_their_categories`
   - (existing `rename`/`create` tests updated for the new signatures, no new cycle)
2. Infrastructure tests
   - SqliteCategoryRepository
     - `it_should_return_the_color_after_inserting_a_category`
     - `it_should_update_the_color`
   - sqlite_migrations
     - `it_should_backfill_existing_categories_with_palette_colors_in_creation_order`
3. Web (Vitest)
   - `categoryColors.test.ts`: `defaultColor` least-used/tie; `cardBackground` for 0/1/2/3/4 colors; `pastelFromHue`/`hueOf` round trip within 3 degrees (8-bit hex quantization at this chroma shifts the hue up to ~2.8°, so 1° is unreachable).
   - `CategoryColorPicker.test.tsx`: choosing a swatch; a custom color shows "Custom" checked.
   - `AddCategoryDialog.test.tsx`: next free color preselected; creates with the chosen color.
   - `CategoryDetail.test.tsx`: header tinted; "Change color" saves and the header updates; cards untinted.
   - `SidebarCategories.test.tsx`: dot in the category color.
   - `Home.test.tsx`: one-category card tinted, two-category diagonal split, four-category grey, uncategorized untinted.
