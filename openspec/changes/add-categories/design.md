## Files

Backend:

- `migrations/0011_categories.sql` — `categories` (id, name, created_at; unique index on `name COLLATE NOCASE`) and `category_members` (category_id, member_kind, member_id; PK on all three; index on `(member_kind, member_id)`). `member_kind` is `channel`/`playlist`, leaving room for `video`.
- `src/infrastructure/shared/sqlite_migrations.rs` — registers 0011.
- `src/domain/category/mod.rs` — new aggregate module.
- `src/domain/category/category.rs` — `Category` entity (name + members) and its transitions.
- `src/domain/category/category_id.rs` — generated UUID identifier (like `VideoRecordId`).
- `src/domain/category/category_name.rs` — trimmed, 1–60 chars, case-insensitive sameness; exhaustive VO tests.
- `src/domain/category/category_member.rs` — `CategoryMember` enum, one variant per member kind.
- `src/domain/category/category_view.rs` — `CategoryView` and `CategoryViews` read models.
- `src/domain/category/errors.rs` — every category use-case error.
- `src/domain/mod.rs` — `pub mod category`.
- `src/domain/services/category_creator.rs` — create, name uniqueness.
- `src/domain/services/category_updater.rs` — rename and set members (uniqueness, members tracked).
- `src/domain/services/category_deleter.rs` — delete.
- `src/domain/services/category_member_remover.rs` — remove a deleted source from every category.
- `src/domain/services/category_video_collector.rs` — the downloaded `SourcedVideo`s of a category's existing members, channels first; shared by the three read/watch paths below.
- `src/domain/services/category_view_searcher.rs` — `CategoryViews` with per-category counts and the distinct total.
- `src/domain/services/video_searcher.rs` — gains `list_for_category`.
- `src/domain/services/video_watch_state_updater.rs` — gains `mark_category_watched`.
- `src/domain/services/mod.rs` — exports.
- `src/domain/video/errors.rs` — `ListVideosError::CategoryNotFound`, `UpdateWatchStateError::CategoryNotFound`.
- `src/infrastructure/repositories/sqlite_category_repository.rs` — `CategoryRepository` trait + SQLite implementation (aggregate persisted as category row + member rows).
- `src/infrastructure/repositories/mod.rs` — module.
- `src/infrastructure/infrastructure_container.rs` — `category_repository`.
- `src/application/http/categories/mod.rs` — create, list, update, delete, set members, mark watched handlers.
- `src/application/http/categories/dto.rs` — request/response DTOs.
- `src/application/http/videos/mod.rs` — `list_videos_for_category` (returns `HomeVideoResponse`s); error mappers gain the category variants.
- `src/application/http/mod.rs` — `ApiServices` fields + routes.
- `src/application/subscribers/remove_channel_from_categories_on_channel_deleted.rs` — `ChannelDeleted` → remover.
- `src/application/subscribers/remove_playlist_from_categories_on_playlist_deleted.rs` — `PlaylistDeleted` → remover.
- `src/application/subscribers/mod.rs` — registers both next to the existing delete-files subscribers.
- `src/serve.rs` — wires services, collector and remover.

Web (`web/src/`):

- `api/types.ts` — `Category`, `CategoryList`, `CategoryMembers`; mirrors `categories/dto.rs`.
- `api/client.ts` — category calls.
- `api/queries.ts` — `queryKeys.categories`, `queryKeys.categoryVideos`, `useCategories`, `useCategoryVideos`; `useInvalidateLibrary` also invalidates categories.
- `hooks/useWatchProgress.ts` — invalidates categories with channels when the watched state flips.
- `hooks/useSaveSourceCategories.ts` — applies a source's chosen categories (one members PUT per changed category).
- `lib/categories.ts` (+ test) — ordering, membership lookups and changes, the manage sources dialog's chips, filtering and staged summary; pure.
- `test/helpers.tsx` — `aCategory`, `aCategoryList` builders.
- `components/VideoGrid.tsx` — home card grid extracted from `Home.tsx` for reuse.
- `components/Home.tsx` — uses `VideoGrid`.
- `components/SidebarSectionHeading.tsx` — the foldable heading from `foldable-sidebar-sections`, extracted for reuse (no actions slot). Takes an optional `add` control, rendered as a trailing "+" outside the `h3` so the heading's accessible name stays the title; the folded total is a badge styled like the row badges.
- `components/SidebarCategories.tsx` (+ test) — the Categories section, rows, row menu ("Manage sources", "Delete"), delete confirmation.
- `components/Sidebar.tsx` — renders `SidebarCategories` above Channels; uses `SidebarSectionHeading`. Each section (and `SidebarCategories`) shows the full-width add row only when loaded and empty, otherwise passes `add` to the heading.
- `components/AddCategoryDialog.tsx` (+ test) — name-only create dialog; lands on the new category with the manage sources dialog open.
- `components/CategoryDetail.tsx` (+ test) — category view (`/categories/:id`): `DetailHeader` with rename, summary and actions menu; opens `ManageSourcesDialog`, also when arriving with `ManageSourcesState`.
- `components/ManageSourcesDialog.tsx` (+ test) — the chip dialog: search, state filter, staged changes, discard confirmation.
- `components/EditableTitle.tsx` (+ test) — a heading with a "Rename" pencil that turns into an in-place field.
- `components/DetailHeader.tsx` (+ test) — `onSync` optional, `summaryPrefix`, `onRename` (renders `EditableTitle`), `menuItems` passed to `EntryActionsMenu` as leading items; `videos` widened to any array.
- `components/ui/toggle.tsx`, `components/ui/toggle-group.tsx` — shadcn wrappers over the `radix-ui` Toggle and ToggleGroup (already a dependency).
- `components/CategoryPicker.tsx` (+ test) — chip picker with inline create.
- `components/AddChannelDialog.tsx`, `AddPlaylistDialog.tsx`, `EditChannelDialog.tsx`, `EditPlaylistDialog.tsx` (+ tests) — Categories field.
- `App.tsx` — the `/categories/:id` route, add category dialog state. There is no `/categories` route and the Settings menu is unchanged.
- `smoke-tests/tests/categories.spec.js` — every new route through the real binary.

## Types & Signatures

```rust
// src/domain/category/category_id.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CategoryId(String);
impl CategoryId {
    pub fn new_generated() -> Self;
    pub fn new(id: impl Into<String>) -> Result<Self, ValidationError>; // non-blank
    pub fn as_str(&self) -> &str;
}

// src/domain/category/category_name.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryName(String);
impl CategoryName {
    pub const MAX_LENGTH: usize = 60;
    pub fn new(name: impl Into<String>) -> Result<Self, ValidationError>; // trims; 1..=60 chars
    pub fn as_str(&self) -> &str;
    /// Equal ignoring case.
    pub fn is_same_as(&self, other: &CategoryName) -> bool;
}

// src/domain/category/category_member.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CategoryMember {
    Channel(ChannelHandle),
    Playlist(PlaylistId),
}

// src/domain/category/category.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: CategoryId,
    pub name: CategoryName,
    /// Each member once, in the order first added.
    pub members: Vec<CategoryMember>,
    pub created_at: DateTime<Utc>,
}
impl Category {
    pub fn create(name: CategoryName, now: DateTime<Utc>) -> Self; // generated id, no members
    pub fn with_name(self, name: CategoryName) -> Self;
    /// Replaces the members, keeping each once (first occurrence wins).
    pub fn with_members(self, members: Vec<CategoryMember>) -> Self;
    pub fn without_member(self, member: &CategoryMember) -> Self;
    pub fn has_member(&self, member: &CategoryMember) -> bool;
}

// src/domain/category/category_view.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryView {
    pub id: CategoryId,
    pub name: CategoryName,
    pub unwatched_count: usize,
    /// Existing members only.
    pub channels: Vec<ChannelHandle>,
    pub playlists: Vec<PlaylistId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryViews {
    pub categories: Vec<CategoryView>,
    pub unwatched_total: usize,
}

// src/domain/category/errors.rs
pub enum CreateCategoryError { NameTaken(CategoryName), Repository(anyhow::Error) }
pub enum UpdateCategoryError {
    NotFound(CategoryId),
    NameTaken(CategoryName),
    MemberNotTracked(CategoryMember),
    Repository(anyhow::Error),
}
pub enum DeleteCategoryError { NotFound(CategoryId), Repository(anyhow::Error) }

// src/domain/video/errors.rs (added variants)
ListVideosError::CategoryNotFound(CategoryId)
UpdateWatchStateError::CategoryNotFound(CategoryId)
```

```rust
// src/infrastructure/repositories/sqlite_category_repository.rs
pub trait CategoryRepository: Send + Sync {
    fn find(&self, id: &CategoryId) -> anyhow::Result<Option<Category>>;
    /// By name ignoring case, then creation order.
    fn list(&self) -> anyhow::Result<Vec<Category>>;
    fn insert(&self, category: &Category) -> anyhow::Result<()>;
    /// Name and the full member set, in one transaction.
    fn update(&self, category: &Category) -> anyhow::Result<()>;
    /// The category and its member rows.
    fn delete(&self, id: &CategoryId) -> anyhow::Result<()>;
}
pub struct SqliteCategoryRepository { db: Database }
```

```rust
// src/domain/services/*.rs — public contracts (structs hold Arc<dyn …> ports)
pub trait CategoryCreatorApi: Send + Sync {
    fn create(&self, name: CategoryName) -> Result<Category, CreateCategoryError>;
}
// CategoryCreator::new(repository: Arc<dyn CategoryRepository>, clock: Arc<dyn Clock>)

pub trait CategoryUpdaterApi: Send + Sync {
    /// Another category with the same name ignoring case → NameTaken.
    fn rename(&self, id: &CategoryId, name: CategoryName) -> Result<(), UpdateCategoryError>;
    /// Every member must be a tracked channel/playlist; the first that is not → MemberNotTracked.
    fn set_members(&self, id: &CategoryId, members: Vec<CategoryMember>) -> Result<(), UpdateCategoryError>;
}
// CategoryUpdater::new(repository, channel_repository, playlist_repository)

pub trait CategoryDeleterApi: Send + Sync {
    fn delete(&self, id: &CategoryId) -> Result<(), DeleteCategoryError>;
}
// CategoryDeleter::new(repository)

pub trait CategoryMemberRemoverApi: Send + Sync {
    /// Removes `member` from every category holding it; categories stay.
    fn remove_from_all(&self, member: &CategoryMember) -> anyhow::Result<()>;
}
// CategoryMemberRemover::new(repository)

pub trait CategoryVideoCollectorApi: Send + Sync {
    /// Downloaded videos of the category's members that still exist, once per
    /// member, channel members first. Members already deleted are skipped.
    fn downloaded_videos(&self, category: &Category) -> anyhow::Result<Vec<SourcedVideo>>;
}
// CategoryVideoCollector::new(channel_repository, channel_video_repository,
//     playlist_repository, playlist_video_repository, video_repository)

pub trait CategoryViewSearcherApi: Send + Sync {
    /// Per-category distinct unwatched counts and the distinct total across all.
    fn search_all(&self) -> anyhow::Result<CategoryViews>;
}
// CategoryViewSearcher::new(repository, channel_repository, playlist_repository, collector)

// VideoSearcherApi (added)
/// Downloaded videos of the category's members, once per YouTube video
/// (channel copy preferred), newest sync first.
fn list_for_category(&self, id: &CategoryId) -> Result<Vec<HomeVideoView>, ListVideosError>;
// VideoSearcher::new(..existing, category_repository, category_video_collector)

// VideoWatchStateUpdaterApi (added)
/// Marks every downloaded video of the category watched, every copy of each.
fn mark_category_watched(&self, id: &CategoryId) -> Result<(), UpdateWatchStateError>;
// VideoWatchStateUpdater::new(..existing, category_repository, category_video_collector)
```

```rust
// src/application/http/categories/dto.rs
#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest { #[serde(default)] pub name: Option<String> }
#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest { #[serde(default)] pub name: Option<String> }
#[derive(Debug, Deserialize)]
pub struct SetCategoryMembersRequest {
    #[serde(default)] pub channels: Vec<String>,
    #[serde(default)] pub playlists: Vec<String>,
}
#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryResponse { pub id: String, pub name: String, pub channels: Vec<String>, pub playlists: Vec<String> }
impl From<Category> for CategoryResponse;
#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryListItemResponse {
    pub id: String, pub name: String, pub unwatched_count: usize,
    pub channels: Vec<String>, pub playlists: Vec<String>,
}
impl From<CategoryView> for CategoryListItemResponse;
#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryListResponse { pub categories: Vec<CategoryListItemResponse>, pub unwatched_total: usize }
impl From<CategoryViews> for CategoryListResponse;

// src/application/http/categories/mod.rs
pub async fn create_category(State(CategoryCreator), Json(CreateCategoryRequest))
    -> Result<(StatusCode, Json<CategoryResponse>), ApiError>;           // 201 | 400 | 409
pub async fn list_categories(State(CategoryViewSearcher))
    -> Result<Json<CategoryListResponse>, ApiError>;                     // 200
pub async fn update_category(State(CategoryUpdater), Path(String), Json(UpdateCategoryRequest))
    -> Result<StatusCode, ApiError>;                                     // 204 | 400 | 404 | 409
pub async fn delete_category(State(CategoryDeleter), Path(String))
    -> Result<StatusCode, ApiError>;                                     // 204 | 404
pub async fn set_category_members(State(CategoryUpdater), Path(String), Json(SetCategoryMembersRequest))
    -> Result<StatusCode, ApiError>;                                     // 204 | 400 | 404
pub async fn mark_category_watched(State(VideoWatchStateUpdater), Path(String))
    -> Result<StatusCode, ApiError>;                                     // 204 | 404

// src/application/http/videos/mod.rs
pub async fn list_videos_for_category(State(VideoSearcher), Path(String))
    -> Result<Json<Vec<HomeVideoResponse>>, ApiError>;                   // 200 | 404

// routes (src/application/http/mod.rs)
// POST/GET   /categories
// PATCH/DELETE /categories/{id}
// PUT        /categories/{id}/members
// GET        /categories/{id}/videos
// POST       /categories/{id}/watched

// src/application/subscribers/remove_*_from_categories_on_*_deleted.rs
pub struct RemoveChannelFromCategoriesOnChannelDeleted { remover: CategoryMemberRemover }
pub struct RemovePlaylistFromCategoriesOnPlaylistDeleted { remover: CategoryMemberRemover }
impl EventSubscriber for … { fn handle(&self, payload: &str) -> anyhow::Result<()>; }
// subscribers::registry(.., category_member_remover: CategoryMemberRemover)
```

Error mapping: `NameTaken` → 409, `NotFound`/`CategoryNotFound` → 404, `MemberNotTracked` and `ValidationError` → 400, `Repository` → 500. A missing `name` uses `required(.., MISSING_CATEGORY_NAME)`.

```ts
// web/src/api/types.ts
export interface Category { id: string; name: string; unwatched_count: number; channels: string[]; playlists: string[] }
export interface CategoryList { categories: Category[]; unwatched_total: number }
export interface CategoryMembers { channels: string[]; playlists: string[] }
export interface CreatedCategory { id: string; name: string; channels: string[]; playlists: string[] }

// web/src/api/client.ts
export function listCategories(): Promise<CategoryList>
export function createCategory(name: string): Promise<CreatedCategory>
export function renameCategory(id: string, name: string): Promise<void>
export function deleteCategory(id: string): Promise<void>
export function setCategoryMembers(id: string, members: CategoryMembers): Promise<void>
export function listCategoryVideos(id: string): Promise<HomeVideo[]>
export function markCategoryWatched(id: string): Promise<void>

// web/src/api/queries.ts
queryKeys.categories                 // ['categories']
queryKeys.categoryVideos(id)         // ['categories', id, 'videos']
export function useCategories(): UseQueryResult<CategoryList, Error>
export function useCategoryVideos(id: string): UseQueryResult<HomeVideo[], Error>

// web/src/lib/categories.ts
export type SourceRef = { kind: 'channel' | 'playlist'; id: string }
export function orderCategories(categories: Category[]): Category[]          // unread first, most first, then name ignoring case
export function memberCount(category: Category): number
export function categoryIdsOf(categories: Category[], source: SourceRef): string[]
export function withMember(category: Category, source: SourceRef): CategoryMembers
export function withoutMember(category: Category, source: SourceRef): CategoryMembers
export function withMembers(category: Category, sources: SourceRef[]): CategoryMembers
export interface SourceChip { source: SourceRef; name: string; avatarFilename: string | null }
export type SourceFilter = 'all' | 'in' | 'out'
/** Every tracked source as a chip, each kind by name ignoring case. */
export function sourceChips(channels: ChannelListItem[] | undefined, playlists: PlaylistListItem[] | undefined):
  { channels: SourceChip[]; playlists: SourceChip[] }
export function membersOf(category: Category): CategoryMembers
export function isIn(members: CategoryMembers, source: SourceRef): boolean
export function toggled(members: CategoryMembers, source: SourceRef): CategoryMembers
/** Chips matching the search (name, ignoring case) and the state filter, order kept. */
export function visibleChips(chips: SourceChip[], staged: CategoryMembers, search: string, filter: SourceFilter): SourceChip[]
export function stagedSummary(saved: CategoryMembers, staged: CategoryMembers): { added: number; removed: number }
/** The members PUT each category needs so `source` is in exactly `chosenIds`. */
export function membershipChanges(categories: Category[], source: SourceRef, chosenIds: string[]):
  { id: string; members: CategoryMembers }[]

// web/src/hooks/useSaveSourceCategories.ts
export function useSaveSourceCategories(): (source: SourceRef, chosenIds: string[]) => Promise<void>

// web/src/components
export function SidebarSectionHeading(props: { title: string; folded: boolean; onToggleFolded: () => void;
  headingActive: boolean; foldedTotal?: number; add?: { label: string; onClick: () => void } }): JSX.Element
export function SidebarCategories(props: { onAddCategory: () => void; onClose: () => void }): JSX.Element
export function AddCategoryDialog(props: { open: boolean; onOpenChange: (open: boolean) => void }): JSX.Element
export function CategoryDetail(): JSX.Element                // reads :id and ManageSourcesState
/** Router state that opens the manage sources dialog on arrival; consumed once (replaced) so reload/back don't reopen it. */
export interface ManageSourcesState { manageSources: true }
export function ManageSourcesDialog(props: { category: Category; open: boolean;
  onOpenChange: (open: boolean) => void }): JSX.Element
export function EditableTitle(props: { name: string; onRename: (name: string) => Promise<void> }): JSX.Element
// DetailHeaderProps (changed): onSync?: () => Promise<void>; summaryPrefix?: string;
//   onRename?: (name: string) => Promise<void>; menuItems?: ReactNode; videos: readonly unknown[] | undefined
export function CategoryPicker(props: { value: string[]; onChange: (ids: string[]) => void }): JSX.Element
export function VideoGrid(props: { videos: HomeVideo[]; showProgress?: boolean }): JSX.Element
```

## Call Stack

Create category:
1. `POST /api/categories {name}` → `create_category` → `CategoryName::new(required(name)?)?`
2. `run_blocking` → `CategoryCreator::create(name)` → `repository.list()` → any `is_same_as` → `NameTaken`
3. `Category::create(name, clock.now())` → `repository.insert(&category)` → `201 CategoryResponse::from(category)`

List categories:
1. `GET /api/categories` → `list_categories` → `CategoryViewSearcher::search_all()`
2. `repository.list()`; `channel_repository.list()`, `playlist_repository.list()` (existing member filter)
3. per category → `collector.downloaded_videos(&category)` → unwatched → `HashSet<VideoId>` → count; total = union of sets
4. `CategoryListResponse::from(views)`

Rename:
1. `PATCH /api/categories/{id} {name}` → `CategoryId::new(id)?`, `CategoryName::new(required(name)?)?`
2. `CategoryUpdater::rename(&id, name)` → `find_category` (`NotFound`) → `repository.list()` other with `is_same_as` → `NameTaken`
3. `category.with_name(name)` → `repository.update(&category)` → `204`

Set members:
1. `PUT /api/categories/{id}/members {channels, playlists}` → `ChannelHandle::new` / `PlaylistId::new` each → `Vec<CategoryMember>`
2. `CategoryUpdater::set_members(&id, members)` → `find_category` → each member: `channel_repository.find` / `playlist_repository.find` → `None` → `MemberNotTracked`
3. `category.with_members(members)` → `repository.update(&category)` → `204`

Delete:
1. `DELETE /api/categories/{id}` → `CategoryDeleter::delete(&id)` → `find` (`NotFound`) → `repository.delete(&id)` → `204`

Category videos:
1. `GET /api/categories/{id}/videos` → `VideoSearcher::list_for_category(&id)` → `category_repository.find` (`CategoryNotFound`)
2. `collector.downloaded_videos(&category)` (channels first) → `once_per_youtube_video` → sort by `created_at` desc → `HomeVideoView::from`
3. `Json(Vec<HomeVideoResponse>)`

Mark category watched:
1. `POST /api/categories/{id}/watched` → `VideoWatchStateUpdater::mark_category_watched(&id)` → `find` (`CategoryNotFound`)
2. `collector.downloaded_videos(&category)` → unwatched youtube ids (set) → `mark_copies_watched(youtube_id, now)` each → `204`

Source deleted:
1. `ChannelDeleter::delete` publishes `ChannelDeleted` (unchanged) → `DomainEventsConsumer`
2. `RemoveChannelFromCategoriesOnChannelDeleted::handle(payload)` → `ChannelHandle::new(channel_id)` (invalid → `Ok(())`)
3. `CategoryMemberRemover::remove_from_all(&CategoryMember::Channel(handle))` → `repository.list()` → holding ones → `without_member` → `repository.update`
4. (playlist: same with `PlaylistDeleted` / `PlaylistId`)

Web — save a source's categories (add/edit dialogs):
1. dialog save → (add) `createChannel`/`createPlaylist` or (edit) settings PATCH only when settings changed
2. `useSaveSourceCategories()(source, chosenIds)` → `membershipChanges(categories, source, chosenIds)` → `setCategoryMembers(id, members)` each → invalidate `categories`
3. failure after the source saved → dialog shows "added/saved, but its categories were not saved"

Web — manage sources:
1. open (⋮ "Manage sources", empty-state control, or arrival with `ManageSourcesState` from the sidebar row / `AddCategoryDialog`) → `staged = membersOf(category)`
2. chip toggle / Enter on the only match → `staged = toggled(staged, source)`; `stagedSummary(membersOf(category), staged)` drives the footer and Save
3. Save → `setCategoryMembers(category.id, staged)` (one PUT, full set) → invalidate library → close; failure → message, dialog stays
4. Cancel/Esc/outside click with a non-zero summary → `ConfirmDialog` "Discard changes?" → close

Web — rename:
1. pencil → field (focused, selected) → Enter/blur: unchanged → close; else `renameCategory(id, name)` → invalidate library → close
2. failure → message under the field, field stays open; Escape → close showing the saved name

Decisions:
- One searchable chip set instead of two transfer panes: wraps on any width, one search, state filter replaces the second pane.
- Chips are a `ToggleGroup type="multiple"` per kind: roving arrow-key focus and `aria-pressed` for free; Tab moves between groups, not through every chip.
- Order never changes on toggle, so the chip under the pointer stays put.
- Staged edits with one PUT: matches the members endpoint (full set), gives Cancel a meaning and removes the save-one-change-at-a-time locking.

## Test Plan

1. Acceptance tests (Rust, application layer)
   - `http/categories` › `it_should_create_a_category` — 201 with generated id (read back from the repository), trimmed name, no members; row stored
   - `http/categories` › `it_should_fail_to_create_if_name_missing` — 400 missing-name message
   - `http/categories` › `it_should_fail_to_create_if_name_invalid` — 400 from `CategoryName`
   - `http/categories` › `it_should_fail_to_create_if_name_taken_ignoring_case` — 409; still one category
   - `http/categories` › `it_should_list_categories_by_name_ignoring_case` — order and member lists
   - `http/categories` › `it_should_count_distinct_unwatched_videos_per_category` — channel + playlist members, shared video once, non-downloaded ignored
   - `http/categories` › `it_should_count_the_total_once_across_categories` — video in two categories counts once in `unwatched_total`
   - `http/categories` › `it_should_skip_members_already_deleted` — member row whose channel is gone: not listed, not counted
   - `http/categories` › `it_should_rename_a_category`
   - `http/categories` › `it_should_rename_a_category_to_a_case_variant_of_its_name`
   - `http/categories` › `it_should_fail_to_rename_if_name_taken`
   - `http/categories` › `it_should_fail_to_rename_a_missing_category` — 404
   - `http/categories` › `it_should_delete_a_category_keeping_its_sources` — channels, playlists, videos untouched
   - `http/categories` › `it_should_fail_to_delete_a_missing_category` — 404
   - `http/categories` › `it_should_set_the_members_of_a_category`
   - `http/categories` › `it_should_replace_the_members_of_a_category`
   - `http/categories` › `it_should_keep_a_repeated_member_once`
   - `http/categories` › `it_should_let_a_source_be_in_several_categories`
   - `http/categories` › `it_should_fail_to_set_members_if_a_source_is_not_tracked` — 400 naming it; members unchanged
   - `http/categories` › `it_should_fail_to_set_members_if_a_handle_is_invalid` — 400 from `ChannelHandle`
   - `http/categories` › `it_should_fail_to_set_members_of_a_missing_category` — 404
   - `http/categories` › `it_should_mark_a_category_watched` — every downloaded video of channel and playlist members watched, copies outside the category too, pending left
   - `http/categories` › `it_should_fail_to_mark_a_missing_category_watched` — 404
   - `http/videos` › `it_should_list_videos_for_a_category` — across members, newest sync first, home-video fields with source
   - `http/videos` › `it_should_list_a_shared_video_once_from_the_channel`
   - `http/videos` › `it_should_list_only_downloaded_videos_for_a_category`
   - `http/videos` › `it_should_list_videos_of_a_playlist_excluded_from_home_for_a_category`
   - `http/videos` › `it_should_list_no_videos_for_a_category_without_members`
   - `http/videos` › `it_should_fail_to_list_videos_for_a_missing_category` — 404
   - `subscribers/remove_channel_from_categories_on_channel_deleted` › `it_should_remove_the_channel_from_every_category` — categories stay, others' members untouched
   - `subscribers/remove_channel_from_categories_on_channel_deleted` › `it_should_ignore_an_invalid_channel_id`
   - `subscribers/remove_playlist_from_categories_on_playlist_deleted` › `it_should_remove_the_playlist_from_every_category`
2. Value object tests
   - `category_name` › accepts a name; trims it; rejects empty; rejects blank; accepts 60 chars; rejects 61; `is_same_as` ignores case
   - `category_id` › accepts an id; rejects blank
3. Infrastructure tests (`sqlite_category_repository`)
   - `it_should_return_the_category_after_inserting_it` — with members, kinds round-trip
   - `it_should_list_categories_by_name_ignoring_case`
   - `it_should_keep_members_in_the_order_first_added`
   - `it_should_replace_members_on_update`
   - `it_should_delete_the_category_and_its_members` — other categories' members untouched
4. Web unit and component tests (Vitest)
   - `lib/categories` › orders unread first, most first, ties and caught-up by name ignoring case
   - `lib/categories` › finds the categories of a source
   - `lib/categories` › computes membership changes: adds, removes, leaves unchanged ones out
   - `lib/categories` › adds and removes members without duplicates
   - `SidebarCategories` › lists every category above channels with badges, in unread order
   - `SidebarCategories` › opens a category and marks its row active
   - `SidebarCategories` › folds with the unwatched total badge and marks the heading active on a category route
   - `SidebarCategories` › manages sources and deletes from the row menu, with the member-count confirmation
   - `SidebarCategories` › is not filtered by the sidebar search
   - `Sidebar` › offers a labelled add row in an empty section and a heading + once it has items
   - `Sidebar` › shows the channels' unwatched total badge only while folded and above 0
   - `AddCategoryDialog` › creates a category and opens it with the manage sources dialog
   - `AddCategoryDialog` › disables create for a blank name and shows a taken-name error
   - `lib/categories` › builds chips per kind by name ignoring case, channels with avatars
   - `lib/categories` › filters chips by search and state keeping their order
   - `lib/categories` › toggles a source and summarises staged additions and removals
   - `EditableTitle` › renames on Enter and on blur, cancels on Escape, closes unchanged without saving
   - `EditableTitle` › keeps the field open with the message when the name is rejected
   - `DetailHeader` › shows the summary prefix and menu items, and no Sync without `onSync`
   - `CategoryDetail` › shows the summary and video cards newest first
   - `CategoryDetail` › marks all watched and deletes from the actions menu
   - `CategoryDetail` › renames from the header
   - `CategoryDetail` › explains an empty category, whose control opens the dialog, and an unknown one
   - `CategoryDetail` › opens the dialog on arrival with `ManageSourcesState`, once
   - `ManageSourcesDialog` › shows channels and playlists as chips pressed by membership
   - `ManageSourcesDialog` › stages toggles, summarises them and saves the full set at once
   - `ManageSourcesDialog` › filters by search and state, and Enter toggles the only match
   - `ManageSourcesDialog` › disables Save with nothing staged and asks before discarding
   - `ManageSourcesDialog` › keeps staged changes when saving fails
   - `ManageSourcesDialog` › says when nothing is tracked
   - `CategoryPicker` › picks, removes and creates categories
   - `AddChannelDialog` › adds a channel into the chosen categories, and reports categories that failed to save
   - `AddPlaylistDialog` › adds a playlist into the chosen categories
   - `EditChannelDialog` › saves a change of categories alone
   - `EditPlaylistDialog` › saves a change of categories alone
   - `useWatchProgress` › refetches categories when a report flips the watched state
5. Smoke test (`smoke-tests/tests/categories.spec.js`)
   - create (lands in manage sources), add sources with chips, rename in place, mark all watched, assign from the add channel dialog, delete from the actions menu — covering every new route
