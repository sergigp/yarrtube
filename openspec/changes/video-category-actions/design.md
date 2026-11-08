# Design

Product flow, chip states and what each action removes: see `proposal.md`.
Behaviour: see `specs/video-category-editing`, `specs/web-ui`,
`specs/category-crud`. Builds on `categorize-videos-on-reconcile`'s
`VideoCategoryVerdict`, `VideoCategoryVerdictRepository`,
`VideoCategoryAssigner::apply_verdict`.

## Diagrams

### Re-run a source, end to end

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant W as EntryActionsMenu
    participant H as DELETE /api/channels/:handle/ai-verdicts
    participant E as VideoCategoryEditor
    participant R as VerdictRepository
    participant A as VideoCategoryAssigner
    participant T as TaskRepository
    participant S as Sync + CategorizeVideoTask

    U->>W: "Re-run AI categorization", confirm
    W->>H: DELETE
    H->>E: rerun_source(Channel(handle))
    E->>R: delete_ai_for_videos(source's record ids)
    R-->>E: records that had AI verdicts
    E->>A: recompute(records)
    A-->>A: lock, source ∪ yes verdicts (user ones only now)
    A-->>U: VideoCategoriesChanged (Plex labels, UI refetch)
    E->>T: schedule ReconcileChannel
    H-->>W: 204
    Note over T,S: later, per video: sync finds pending pairs<br/>(none if categorization is off: a clear)
    T->>S: sync_video → categorize_video
```

## Files

**Created**
- `src/domain/video_category/video_category_choice.rs`: `VideoCategoryChoice`, `CategoryOrigin`; one row of the dialog.
- `src/domain/video_category/video_category_choices.rs`: `VideoCategoryChoices`, the rows plus whether the source has categorization on.
- `src/domain/video_category/errors.rs`: `EditVideoCategoriesError`.
- `src/domain/services/video_category_choice_searcher.rs`: reads a video's choices.
- `src/domain/services/video_category_editor.rs`: user verdicts and AI re-runs.
- `src/application/http/videos/category_dto.rs`: request/response DTOs for the video routes.
- `web/src/components/VideoCategoriesDialog.tsx` (+ `.test.tsx`): the Edit categories dialog; its button reads "Clear AI categories" when the source has categorization off.
- `web/src/components/VideoCategoriesEditor.tsx`, `web/src/hooks/useEditVideoCategories.ts`: host the one dialog above every view, so it stays open when the card that opened it leaves its list.
- `src/application/http/test_support.rs`: test helpers shared by the HTTP test modules.
- `e2e-tests/tests/videoCategories.spec.js`: edit a video's categories by hand.

**Changed**
- `src/infrastructure/repositories/sqlite_video_category_verdict_repository.rs`: scoped AI deletes, count per category.
- `src/domain/services/video_category_assigner.rs`: `recompute` for many records, one event.
- `src/domain/category/category_view.rs`, `src/domain/services/category_view_searcher.rs`: `ai_verdict_count`.
- `src/application/http/videos/dto.rs`: `record_id` on `VideoResponse`, `HomeVideoResponse`, `CategoryVideoResponse`. The web only knew the YouTube `id`, and one YouTube video can have several copies (records), so the record id is what names a copy in the video routes.
- `src/domain/video/home_video_view.rs`: `HomeVideoView.record_id` (the other views already hold the `Video`).
- `src/application/http/videos/mod.rs`: `GET/PUT /videos/{id}/categories[/{category_id}]`, `DELETE /videos/{id}/ai-verdicts`.
- `src/application/http/{channels,playlists,categories}/mod.rs`: `DELETE …/ai-verdicts`.
- `src/application/http/categories/dto.rs`: `ai_verdict_count`.
- `src/application/http/mod.rs`, `src/serve.rs`, `src/domain/services/mod.rs`: routes and wiring.
- `web/src/api/{types,client,queries}.ts`, `web/src/test/helpers.tsx`: DTO mirror, calls, `aVideoCategoryChoice`, `ai_verdict_count` in `aCategory`, `record_id` on `Video`/`HomeVideo` and in `aVideo`/`aHomeVideo`.
- `web/src/components/VideoActionsMenu.tsx` (+ test): "Edit categories…" item; new `recordId` and `sourceName` props, passed to the dialog (`videoId`, the YouTube id, stays for "Mark as watched").
- `web/src/components/VideoGrid.tsx`: passes `video.record_id`, `video.source.name`.
- `web/src/components/VideoListPane.tsx`: new `sourceNameOf(video)` prop (like `basePathOf`), passes `video.record_id` and it; `ChannelDetail`, `PlaylistDetail` give the source's name, `CategoryPlayer` `item.source.name`.
- `web/src/components/DetailHeader.tsx` (+ test): source item + confirmation, via an `aiCategorization` prop given by `ChannelDetail` / `PlaylistDetail`.
- `web/src/main.tsx`: wraps `App` in `VideoCategoriesEditor`.
- `web/src/components/CategoryDetail.tsx` (+ test): a category without sources lists the videos set by hand or by the AI, and can mark them watched.
- `src/domain/services/video_categorizer.rs` (+ tests in `src/application/tasks/categorize_video_task.rs`): stores an AI answer only while the pair is `still_pending`.
- `web/src/components/CategorySettingsDialog.tsx` (+ test): "Re-evaluate N videos" + confirmation, disabled while the prompt has unsaved edits.
- `doc/CATEGORIES.md`, `doc/AI.md`: editing by hand and re-running.

## Types & Signatures

```rust
// domain/video_category/video_category_choice.rs
pub enum CategoryOrigin { Source, Ai, User }          // as_str: "source" | "ai" | "user"
pub struct VideoCategoryChoice {
    pub category: Category,                           // id, name, color
    pub belongs: bool,
    pub from: Option<CategoryOrigin>,                 // Source wins over any verdict
    pub verdict: Option<Verdict>,
}
impl VideoCategoryChoice {
    /// Every category, name order, from the source's membership and the video's verdicts.
    pub fn list(categories: &[Category], source: &CategoryMember,
                verdicts: &[VideoCategoryVerdict]) -> Vec<Self>;
}

// domain/video_category/video_category_choices.rs
pub struct VideoCategoryChoices {
    pub choices: Vec<VideoCategoryChoice>,
    pub categorization_enabled: bool,                 // the video's source has categorization on
}

// domain/video_category/errors.rs
pub enum EditVideoCategoriesError {
    VideoNotFound, CategoryNotFound, SourceNotFound,
    FromSource(CategoryMember),                       // "channel" | "playlist" in the message
    Repository(anyhow::Error),
}

// domain/video_category/video_category_verdict.rs
impl VideoCategoryVerdict {
    pub fn by_user(video_id: VideoRecordId, category_id: CategoryId, belongs: bool,
                   now: DateTime<Utc>) -> Self;
}

// domain/services/video_category_choice_searcher.rs
pub trait VideoCategoryChoiceSearcherApi: Send + Sync {
    fn search(&self, video_id: &VideoRecordId) -> Result<VideoCategoryChoices, EditVideoCategoriesError>;
}

// domain/services/video_category_editor.rs
pub trait VideoCategoryEditorApi: Send + Sync {
    /// User verdict via `apply_verdict`; FromSource when the source is a member.
    fn set(&self, video_id: &VideoRecordId, category_id: &CategoryId, belongs: bool)
        -> Result<(), EditVideoCategoriesError>;
    /// Deletes AI verdicts, recomputes, schedules SyncVideo.
    fn rerun_video(&self, video_id: &VideoRecordId) -> Result<(), EditVideoCategoriesError>;
    /// Same for every record of the source; schedules its reconcile pass.
    fn rerun_source(&self, source: &CategoryMember) -> Result<(), EditVideoCategoriesError>;
    /// Deletes the category's AI verdicts, recomputes and syncs the records that had one.
    fn rerun_category(&self, category_id: &CategoryId) -> Result<(), EditVideoCategoriesError>;
}
pub struct VideoCategoryEditor {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    category_repository: Arc<dyn CategoryRepository>,
    channel_repository: Arc<dyn ChannelRepository>,     // rerun_source: 404 for an untracked source
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    video_category_assigner: VideoCategoryAssigner,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

// domain/services/video_category_assigner.rs
pub trait VideoCategoryAssignerApi: Send + Sync {
    /* … */
    /// Under the lock: each record to source ∪ yes verdicts; one event for those that changed.
    fn recompute(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<()>;
}

// domain/services/video_categorizer.rs (private)
/// Under the assigner's lock, before storing an AI answer or an unavailable:
/// the pair still has no verdict, and the settings and prompt are those the job started with.
fn still_pending(&self, video: &Video, job: &CategorizationJob, category: &Category) -> anyhow::Result<bool>;

// infrastructure/repositories/sqlite_video_category_verdict_repository.rs
pub trait VideoCategoryVerdictRepository: Send + Sync {
    /* … */
    /// Deletes decided_by = AI rows; returns the records that had one.
    fn delete_ai_for_videos(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoRecordId>>;
    fn delete_ai_for_category(&self, category_id: &CategoryId) -> anyhow::Result<Vec<VideoRecordId>>;
    fn count_ai_by_category(&self) -> anyhow::Result<HashMap<CategoryId, u64>>;
}

// domain/category/category_view.rs
pub struct CategoryView { /* … */ pub ai_verdict_count: u64 }

// domain/video/home_video_view.rs
pub struct HomeVideoView { /* … */ pub record_id: VideoRecordId }

// application/http/videos/dto.rs
// VideoResponse, HomeVideoResponse, CategoryVideoResponse gain `record_id: String`

// application/http/videos/category_dto.rs
pub struct VideoCategoryChoiceResponse { id: String, name: String, color: String, belongs: bool,
    from: Option<String>, verdict: Option<String> }
pub struct VideoCategoriesResponse { categorization_enabled: bool, categories: Vec<VideoCategoryChoiceResponse> }
pub struct SetVideoCategoryRequest { belongs: Option<serde_json::Value> }   // 400 unless bool
pub const INVALID_BELONGS: &str = "belongs must be true or false";

// application/http handlers
pub async fn list_video_categories(State(..), Path(id)) -> Result<Json<VideoCategoriesResponse>, ApiError>;
pub async fn set_video_category(State(..), Path((id, category_id)), Json(SetVideoCategoryRequest))
    -> Result<Json<VideoCategoriesResponse>, ApiError>;
pub async fn delete_video_ai_verdicts(State(..), Path(id)) -> Result<StatusCode, ApiError>;
pub async fn delete_channel_ai_verdicts(State(..), Path(handle)) -> Result<StatusCode, ApiError>;
pub async fn delete_playlist_ai_verdicts(State(..), Path(id)) -> Result<StatusCode, ApiError>;
pub async fn delete_category_ai_verdicts(State(..), Path(id)) -> Result<StatusCode, ApiError>;
```

```ts
// api/types.ts
type CategoryOrigin = 'source' | 'ai' | 'user'
type Verdict = 'yes' | 'no' | 'unavailable'
interface VideoCategoryChoice { id: string; name: string; color: string; belongs: boolean
  from: CategoryOrigin | null; verdict: Verdict | null }
interface VideoCategories { categorization_enabled: boolean; categories: VideoCategoryChoice[] }
interface Category { /* … */ ai_verdict_count: number }
interface Video { /* … */ record_id: string }      // CategoryVideo inherits it
interface HomeVideo { /* … */ record_id: string }

// api/client.ts
// recordId: the copy's `record_id`, never the YouTube `id`
getVideoCategories(recordId: string): Promise<VideoCategories>
setVideoCategory(recordId: string, categoryId: string, belongs: boolean): Promise<VideoCategories>
deleteVideoAiVerdicts(recordId: string): Promise<void>
deleteSourceAiVerdicts(source: { kind: 'channel' | 'playlist'; id: string }): Promise<void>
deleteCategoryAiVerdicts(categoryId: string): Promise<void>

// api/queries.ts
useVideoCategories(recordId: string, enabled: boolean)   // ['videos', recordId, 'categories']
// setting a category writes the PUT response into it; re-running invalidates it; both invalidate the library (category lists, feeds)

// components/VideoCategoriesDialog.tsx
// components/VideoActionsMenu.tsx: props gain { recordId: string; sourceName: string }
// components/VideoListPane.tsx: props gain { sourceNameOf: (video: T) => string }
function VideoCategoriesDialog(props: { recordId: string; title: string; sourceName: string
  open: boolean; onOpenChange(open: boolean): void }): JSX.Element
```

## Call Stack

Every `{id}` under `/api/videos/{id}/categories` and `/api/videos/{id}/ai-verdicts` is a `VideoRecordId`, the `record_id` of the video listings.

**Open dialog**: `VideoGrid` / `VideoListPane` pass `record_id` + source name → `VideoActionsMenu` "Edit categories…" → `VideoCategoriesDialog` → `GET /api/videos/{record_id}/categories` → `list_video_categories` → `VideoCategoryChoiceSearcher::search(&id)` → `VideoRepository::find`, `VideoOwnerFinder::find`, `CategoryRepository::list`, `VerdictRepository::find_many(&[id])` → `VideoCategoryChoice::list(..)` + `owner.categorization_enabled` → `VideoCategoryChoices`; the dialog's AI button reads "Re-run AI" or "Clear AI categories" from it

**Click a chip**: `setVideoCategory(recordId, categoryId, !belongs)` → `PUT …/categories/{category_id}` → `set_video_category` → `VideoCategoryEditor::set(&id, &category_id, belongs)` → owner, `CategoryRepository::find` (404), `category.has_member(&owner.source)` (409) → `VideoCategoryVerdict::by_user(..)` → `VideoCategoryAssigner::apply_verdict(&verdict, &owner.source)` → handler returns `search(&id)` → `useSetVideoCategory` writes it with `setQueryData`

**Re-run video**: dialog "Re-run AI" → `DELETE /api/videos/{id}/ai-verdicts` → `VideoCategoryEditor::rerun_video(&id)` → `find_source` (404) → `VerdictRepository::delete_ai_for_videos(&[id])` → `assigner.recompute(&affected)` → `TaskRepository::schedule(&Task::SyncVideo { video_id }, now)` → 204 → refetch

**Re-run / clear source**: `EntryActionsMenu` item → `ConfirmDialog` → `DELETE /api/channels/{handle}/ai-verdicts` → `rerun_source(&CategoryMember::Channel(handle))` → `ChannelRepository::find` (404) → `ChannelVideoRepository::list_for_channel` → `delete_ai_for_videos(&ids)` → `recompute(&affected)` → `schedule(&Task::ReconcileChannel { .. })` (playlists: same with `ReconcilePlaylist`)

**Re-evaluate category**: `CategorySettingsDialog` button → `ConfirmDialog` → `DELETE /api/categories/{id}/ai-verdicts` → `rerun_category(&id)` → `CategoryRepository::find` (404) → `delete_ai_for_category(&id)` → `recompute(&affected)` → `schedule(SyncVideo)` per affected record

**AI answer in flight**: `VideoCategorizer::evaluate` / `give_up` → `VideoCategoryAssigner::apply_verdict_if(&verdict, &source, still_pending)` → under the lock: no verdict for the pair (a user one set meanwhile wins) and `unchanged_since` (settings, prompt) → else dropped, the pair stays pending

**Count**: `GET /api/categories` → `CategoryViewSearcher` → `VerdictRepository::count_ai_by_category()` → `CategoryView.ai_verdict_count`

## Test Plan

**1. Behaviour**

HTTP `videos` (`http/videos/mod.rs`)
0a. `it_should_include_the_record_id_when_listing_playlist_videos`.
0b. `it_should_include_the_record_id_when_listing_category_videos`.
0c. `it_should_include_the_record_id_on_home`.
1. `it_should_list_a_videos_categories_with_where_each_comes_from`: source, ai yes/no, user yes, none; name order.
2. `it_should_report_the_source_for_a_member_category_even_with_a_verdict`.
3. `it_should_return_not_found_for_the_categories_of_an_unknown_video`.
4. `it_should_add_a_category_by_hand_replacing_an_ai_no`: user yes, video in category, one event, body is the new list.
5. `it_should_remove_an_ai_category_by_hand`: user no, video out, one event.
6. `it_should_reject_setting_a_category_given_by_the_source`: 409, message names channel / playlist, nothing stored.
7. `it_should_reject_a_missing_or_non_boolean_belongs`: 400.
8. `it_should_return_not_found_for_an_unknown_video_or_category_when_setting`.
9. `it_should_rerun_a_videos_ai_categorization`: AI verdicts gone, user ones kept, AI categories gone, `sync_video` scheduled, 204.
10. `it_should_return_not_found_when_rerunning_an_unknown_video`.
10a. `it_should_say_whether_the_videos_source_has_categorization_on`.

`SyncVideoTask` / `CategorizeVideoTask`

11. `it_should_not_evaluate_a_pair_with_a_user_verdict`: user yes "Tech" → no chat for Tech.
11a. `it_should_not_override_a_user_verdict_set_during_its_call`.
11b. `it_should_not_store_an_answer_to_a_prompt_changed_during_its_call`.

HTTP `channels` / `playlists`

12. `it_should_clear_a_channels_ai_verdicts_keeping_user_ones`: one event naming the changed records, `reconcile_channel` scheduled, 204.
13. `it_should_clear_a_playlists_ai_verdicts_keeping_user_ones`.
14. `it_should_return_not_found_when_clearing_an_unknown_source`.

HTTP `categories`

15. `it_should_rerun_a_categorys_ai_verdicts`: only that category's AI verdicts gone, affected records synced, 204.
16. `it_should_return_not_found_when_rerunning_an_unknown_category`.
17. `it_should_count_ai_verdicts_per_category`: user verdicts not counted, 0 when none.

Web

18. `VideoCategoriesDialog`: `shows locked source chips, AI marks and filled chips`.
19. `VideoCategoriesDialog`: `sets a category by hand when a chip is clicked`.
20. `VideoCategoriesDialog`: `shows the error and keeps the chip when setting fails`.
21. `VideoCategoriesDialog`: `re-runs AI only while AI categorization is available`.
22. `VideoActionsMenu`: `offers Edit categories only while categories are on`.
23. `DetailHeader`: `offers Re-run AI categorization or Clear AI categories after confirmation`.
21a. `VideoCategoriesDialog`: `offers Clear AI categories when the source has categorization off`.
24. `CategorySettingsDialog`: `re-evaluates N videos after confirmation, hidden at 0`.
24a. `CategorySettingsDialog`: `keeps re-evaluate disabled while the prompt has unsaved edits`.

e2e

25. `videoCategories.spec.js`: add and remove a category by hand on a seeded video, chip and category page follow.

**2. Infrastructure**

`SqliteVideoCategoryVerdictRepository`
1. `it_should_delete_only_ai_verdicts_of_the_given_videos_returning_them`.
2. `it_should_delete_only_ai_verdicts_of_a_category_returning_their_videos`.
3. `it_should_count_ai_verdicts_per_category`.

## Decisions

Accepted after `/audit-change` as deliberate; not to be re-flagged:

- `VideoCategoryVerdictRepository::delete_ai_for_videos` / `delete_ai_for_category` delete and return the records that had a verdict, in one statement: finding then deleting would race with a verdict stored in between. `count_ai_by_category` returns a count per category, not verdicts: it feeds a number on every categories poll.
- `CategoryOrigin` lives in `video_category_choice.rs`: it only exists as a field of `VideoCategoryChoice`.
- `VideoCategoryChoice` holds the `Category`: the row is built from it and the DTO reads its id, name and color.
- `src/application/http/videos/category_dto.rs` is a second DTO file for the videos resource: the category routes' wire types, kept apart from the listings'.
- The not-found tests keep their `it_should_return_not_found_*` names (Test Plan 3, 8, 10, 14, 16).
