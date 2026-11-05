## Files

**Created**
- `migrations/0024_ai_categorization.sql`: `categorization_enabled` on `channels`/`playlists`, `ai_prompt` on `categories`.
- `prompts/categorization/v1.md`: shipped wrapping prompt; `prompts/<feature>/<version>.md` is the convention.
- `src/domain/preferences/categorization_settings.rs`: preference group, mirrors `SummarySettings`.
- `src/domain/preferences/categorization_model.rs`: value object, own error message.
- `src/domain/preferences/categorization_prompt.rs`: value object, non-blank plain text.
- `src/domain/category/category_prompt.rs`: value object, trimmed, at most 2000 characters.
- `web/src/components/SourceAiSection.tsx`: the "AI" section of the four source dialogs; replaces `SummariesField.tsx` (deleted) and its test coverage moves with it.
- `web/src/components/CategorySettingsDialog.tsx` (+ `.test.tsx`): `ManageSourcesDialog` renamed, gains the prompt.

**Changed**
- `src/domain/preferences/{preferences,preferences_change,mod}.rs`: `categorization` group, `categorization_available()`.
- `src/domain/category/{category,category_view,mod}.rs`: `ai_prompt`.
- `src/domain/channel/{channel,channel_view,channel_settings_change,errors}.rs`, `src/domain/playlist/{playlist,playlist_view,playlist_settings_change,errors}.rs`: `categorization_enabled`, `CategorizationUnavailable`.
- `src/domain/services/{channel,playlist}_{creator,updater}.rs`: new parameter/field and availability check.
- `src/domain/services/{channel,playlist,category}_view_searcher.rs`: carry the new fields.
- `src/domain/services/category_updater.rs`: `CategoryUpdate.ai_prompt`.
- `src/application/http/{preferences,categories,channels,playlists}/{mod,dto}.rs`, `validation.rs`: request/response fields, errors, `MISSING_CATEGORIZATION`.
- `src/infrastructure/repositories/sqlite_{preferences,category,channel,playlist}_repository.rs`, `src/infrastructure/shared/sqlite_migrations.rs`: persistence.
- `web/src/api/{types,client,queries}.ts`, `web/src/lib/{preferences,aiSettingsForm,channelSettings}.ts`, `web/src/test/helpers.tsx`: DTO mirror, `categorizationAvailable`, fixtures.
- `web/src/components/AiSettingsView.tsx`: Categorization section; `ModelField` and `PromptField` extracted within the file, shared by both sections.
- `web/src/components/{CategoryDetail,SidebarCategories,AddCategoryDialog}.tsx`: rename, `CategorySettingsState`.
- `web/src/components/{AddChannel,EditChannel,AddPlaylist,EditPlaylist}Dialog.tsx`: `SourceAiSection`.
- `e2e-tests/helpers/{seed,fixtures}.js`, `e2e-tests/tests/{channelVideoLimit,moveSources,deepLinks,mobile-sidebar,mobileLayout}.spec.js`: send `categorization_enabled: false`.
- `e2e-tests/tests/aiSettings.spec.js`, `e2e-tests/tests/categories.spec.js`, `e2e-tests/README.md`: exact field names and the rename.
- `web/src/components/DetailHeader.tsx`, `web/src/lib/categories.ts`: comments naming the rename.
- `doc/AI.md`, `doc/CATEGORIES.md`, `doc/ARCHITECTURE.md` if it lists preferences: new settings and the rename.

## Types & Signatures

```sql
ALTER TABLE channels ADD COLUMN categorization_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlists ADD COLUMN categorization_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE categories ADD COLUMN ai_prompt TEXT;
```

```markdown
<!-- prompts/categorization/v1.md -->
You decide whether a YouTube video belongs in a category of a personal video library.
You get the video's title, length and summary when there is one, then the description of the category, written by the user.
Decide only from what you are given, and don't guess when the video clearly doesn't match the description.
```

```rust
// domain/preferences
pub struct CategorizationModel(String);           // default "qwen3:4b"
impl CategorizationModel {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError>;
    pub fn as_str(&self) -> &str;
}

pub struct CategorizationPrompt(String);
impl CategorizationPrompt {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError>;
    pub fn from_stored(value: impl Into<String>) -> Self;
    pub fn shipped() -> Self;
    pub fn as_str(&self) -> &str;
}

#[derive(Default)]
pub struct CategorizationSettings {
    pub enabled: bool,
    pub model: CategorizationModel,
    /// The prompt the user saved; `None` uses the shipped one.
    pub prompt: Option<CategorizationPrompt>,
}
impl CategorizationSettings {
    pub fn effective_prompt(&self) -> CategorizationPrompt;
}

pub struct Preferences { /* … */ pub categorization: CategorizationSettings }
impl Preferences {
    /// AI, categories and categorization all on.
    pub fn categorization_available(&self) -> bool;
}

pub struct PreferencesChange {
    /* … */
    pub categorization_enabled: Option<bool>,
    pub categorization_model: Option<CategorizationModel>,
    /// `Some(None)` goes back to the shipped prompt.
    pub categorization_prompt: Option<Option<CategorizationPrompt>>,
}

// domain/category
pub struct CategoryPrompt(String);
impl CategoryPrompt {
    /// Trimmed; `Ok(None)` when blank.
    pub fn new(value: impl Into<String>) -> Result<Option<Self>, ValidationError>;
    pub fn as_str(&self) -> &str;
}
pub struct Category { /* … */ pub ai_prompt: Option<CategoryPrompt> }
pub struct CategoryView { /* … */ pub ai_prompt: Option<CategoryPrompt> }
impl Category { pub fn with_ai_prompt(self, ai_prompt: Option<CategoryPrompt>) -> Self; }

pub struct CategoryUpdate {
    pub name: Option<CategoryName>,
    pub color: Option<CategoryColor>,
    /// `Some(None)` removes the prompt.
    pub ai_prompt: Option<Option<CategoryPrompt>>,
}

// domain/channel, domain/playlist
pub struct Channel  { /* … */ pub categorization_enabled: bool }
pub struct Playlist { /* … */ pub categorization_enabled: bool }
impl Channel  { pub fn with_categorization_enabled(self, categorization_enabled: bool) -> Self; }
impl Playlist { pub fn with_categorization_enabled(self, categorization_enabled: bool) -> Self; }
pub struct ChannelView  { /* … */ pub categorization_enabled: bool }
pub struct PlaylistView { /* … */ pub categorization_enabled: bool }
pub struct ChannelSettingsChange  { /* … */ pub categorization_enabled: Option<bool> }
pub struct PlaylistSettingsChange { /* … */ pub categorization_enabled: Option<bool> }
pub enum CreateChannelError  { /* … */ CategorizationUnavailable }   // same for Update*, *Playlist*

// channel_creator.rs / playlist_creator.rs: `create` gains `categorization_enabled: bool`
fn ensure_categorization_available(&self, categorization_enabled: bool) -> Result<(), CreateChannelError>;
// channel_updater.rs / playlist_updater.rs
fn ensure_categorization_available(&self, categorization_enabled: Option<bool>) -> Result<(), UpdateChannelError>;

// application/http
pub const MISSING_CATEGORIZATION: &str = "Categorization setting must be true or false (missing)";
// NOTHING_TO_UPDATE (both): "... the reconcile interval, summaries or categorization"
// MISSING_CATEGORY_NAME_AND_COLOR becomes
const NOTHING_TO_UPDATE: &str = "Category name, color or AI prompt is required (all missing)";

UpdatePreferencesRequest { categorization_enabled: Option<bool>, categorization_model: Option<String>,
    #[serde(default, deserialize_with = "present")] categorization_prompt: Option<Option<String>> }
PreferencesResponse { categorization_enabled: bool, categorization_model: String,
    categorization_prompt: String, categorization_prompt_is_default: bool }
UpdateCategoryRequest { ai_prompt: Option<String> }   // "" removes it
CategoryResponse, CategoryListItemResponse { ai_prompt: Option<String> }
Create*Request, Update*Request { categorization_enabled: Option<bool> }
ChannelResponse, ChannelListItemResponse, PlaylistResponse, PlaylistListItemResponse { categorization_enabled: bool }
```

```ts
// api/types.ts
interface Preferences { categorization_enabled: boolean; categorization_model: string
  categorization_prompt: string; categorization_prompt_is_default: boolean }
type PreferencesPatch // + categorization_prompt?: string | null
interface Category { ai_prompt: string | null }
interface ChannelListItem, PlaylistListItem, CreatedChannel… { categorization_enabled: boolean }

// api/client.ts
updateCategory(id, update: { name?: string; color?: string; ai_prompt?: string })

// lib/preferences.ts, api/queries.ts
function categorizationAvailable(preferences: Preferences | undefined): boolean
function useCategorizationAvailable(): boolean

// components/CategoryDetail.tsx
interface CategorySettingsState { editSettings: true }

// components/SourceAiSection.tsx
interface SourceAiOption { checked: boolean; onChange(checked: boolean): void; enabledOnSource?: boolean }
interface SourceAiSectionProps {
  idPrefix: string              // e.g. "add-channel"
  summaries: SourceAiOption
  categorization: SourceAiOption
  onNavigateAway(): void
}
```

`CategoryPrompt` limit: 2000 characters keeps a prompt repeated per category and per video bounded.

## Call Stack

**Save preferences**: `PATCH /api/preferences` → `preferences_change(request)` (builds `CategorizationModel`, `CategorizationPrompt`) → `PreferencesUpdater::update(change)` → `Preferences::apply(change)` → `PreferencesRepository::save(&updated)`. `changes_summaries` ignores `categorization`.

**Set category prompt**: `PATCH /api/categories/:id {ai_prompt}` → `update_category` → `CategoryPrompt::new(text)` → `CategoryUpdater::update(&id, CategoryUpdate { ai_prompt: Some(prompt), .. })` → `Self::applied(category, update)` → `CategoryRepository::update(&category)`.

**Create source**: `POST /api/channels {…, categorization_enabled}` → `required(request.categorization_enabled, MISSING_CATEGORIZATION)` → `ChannelCreator::create(…, summaries_enabled, categorization_enabled)` → `ensure_summaries_available` → `ensure_categorization_available(categorization_enabled)` → `PreferencesRepository::find()` → `Preferences::categorization_available()` → lookup → `Channel::create(…).with_categorization_enabled(…)` → insert. 409 maps from `CategorizationUnavailable`. Playlists alike.

**Update source**: `PATCH /api/channels/:id {categorization_enabled}` → `ChannelUpdater::update_settings(id, change)` → `ensure_categorization_available(change.categorization_enabled)` → apply → save. Playlists alike.

**Edit settings dialog**: `CategoryDetail` (`managing`) → `CategorySettingsDialog` → `save()` → `updateCategory(id, { ai_prompt })` when the field shows and its trimmed text differs from `promptBase` (what it started from or last saved, never the live category) → `setCategoryMembers(id, staged)` when sources differ → close. `invalidateLibrary()` runs after failures too, since the prompt may be saved. On failure stays open; a retry doesn't resend a saved prompt.

**AI page**: `AiSettings` → `changedFields(saved, form)` → `savePreferences(patch)`; `resetCategorizationPrompt` → `submit({ categorization_prompt: null })`.

## Test Plan

Tests that share a name across `channels` and `playlists` are one cycle each.

### 1. Behaviour

**`http/preferences`**
1. `it_should_return_the_categorization_defaults_if_nothing_saved`: off, `qwen3:4b`, shipped prompt, `is_default` true.
2. `it_should_save_the_categorization_settings`: patch returns and stores enabled, model, prompt, `is_default` false.
3. `it_should_go_back_to_the_shipped_categorization_prompt`: `null` drops the saved prompt.
4. `it_should_fail_if_the_categorization_model_is_blank`: 400 with its message.
5. `it_should_fail_if_the_categorization_prompt_is_blank`: 400 with its message.
6. `it_should_keep_unavailable_summaries_when_the_categorization_settings_change`: summary stays `Unavailable`.

**`http/categories`**
7. `it_should_set_the_ai_prompt_of_a_category`: stored trimmed; name, color, members kept.
8. `it_should_remove_the_ai_prompt_of_a_category`: `""` stores `None`.
9. `it_should_keep_the_ai_prompt_when_renaming_a_category`.
10. `it_should_fail_if_the_ai_prompt_is_too_long`: 2001 characters → 400, unchanged.
11. `it_should_list_categories_with_their_ai_prompt`: value and `null`.
12. `it_should_fail_to_update_a_category_with_nothing_to_change`: new message (existing test renamed).

**`http/channels`, then `http/playlists`**
13. `it_should_create_a_channel_with_categorization_enabled` / `…_playlist_…`.
14. `it_should_fail_to_create_a_channel_with_categorization_if_unavailable` / `…_playlist_…`: AI, categories or categorization off → 409, nothing stored, no lookup.
15. `it_should_fail_to_create_if_categorization_missing` (both).
16. `it_should_keep_the_existing_source_when_categorization_differs_on_create` (both): duplicate returns the stored `false`.
17. `it_should_enable_categorization_on_a_channel` / `…_playlist`.
18. `it_should_fail_to_enable_categorization_on_a_channel_if_unavailable` / `…_playlist_…`: other fields unchanged.
19. `it_should_disable_categorization_on_a_channel_while_ai_is_off` / `…_playlist_…`.
20. `it_should_list_channels_with_their_categorization_setting` / `playlists`.

**Web `AiSettingsView.test.tsx`** (existing tests scope "Model" to `within(group 'Summaries')`)
21. shows the Categorization section disabled on a fresh install.
22. hides the Categorization section while categories are off.
23. shows the shipped prompt under Advanced with `Using the default prompt`.
24. saves only the changed categorization fields.
25. resets a saved categorization prompt.
26. lists the server's models in the categorization Model independently of summaries.

**Web `aiSettingsForm.test.ts`, `preferences.test.ts`**
27. `changedFields` includes categorization fields only when changed.
28. `categorizationAvailable` needs AI, categories and categorization.

**Web source dialogs**
29. `AddChannelDialog`: "Summarize videos" and "Categorize videos" sit in the "AI" group.
30. `AddChannelDialog`: creates with `categorization_enabled` false by default, true when checked.
31. `AddPlaylistDialog`: same as 30.
32. `AddChannelDialog`: disabled with a link to AI settings while unavailable; hidden with categories off.
33. `EditChannelDialog`: prefilled, sends only `categorization_enabled` when changed.
34. `EditChannelDialog`: stays enabled to turn off while AI is off.
35. `EditPlaylistDialog`: same as 33.

**Web category settings**
36. `SidebarCategories`: row menu offers "Edit settings", opening "Edit Science settings".
37. `CategoryDetail`: header action "Edit settings"; arrival with `CategorySettingsState` opens it once.
38. `AddCategoryDialog`: creating opens "Edit Chess settings".
39. `CategorySettingsDialog`: shows "AI prompt" prefilled while available.
40. `CategorySettingsDialog`: hides it otherwise and saves sources without `ai_prompt`.
41. `CategorySettingsDialog`: saving only the prompt doesn't send members.
42. `CategorySettingsDialog`: saving both sends both, then closes.
43. `CategorySettingsDialog`: emptying the prompt sends `ai_prompt: ""`.
44. `CategorySettingsDialog`: an edited prompt enables Save and asks to discard on cancel.
45. `CategorySettingsDialog`: a failed save stays open and keeps the text.
46. `CategorySettingsDialog`: a prompt saved elsewhere is left alone when saving the sources.
47. `CategorySettingsDialog`: a hidden prompt changed elsewhere stages nothing.
48. `CategorySettingsDialog`: an edited prompt is left out once its field is hidden.
49. `CategorySettingsDialog`: a pasted prompt is sent whole, for the daemon to judge its length.
50. `CategorySettingsDialog`: says the prompt was saved when only the sources failed, and retries the sources.
51. `CategorySettingsDialog`: says the prompt stays saved when discarding after only the sources failed.

### 2. Infrastructure

**`SqlitePreferencesRepository`**
52. `it_should_round_trip_the_categorization_settings`.
53. `it_should_read_missing_categorization_keys_as_defaults`.

**`SqliteCategoryRepository`**
54. `it_should_round_trip_the_ai_prompt`: insert, update, clear, `list`.

**`SqliteChannelRepository`, `SqlitePlaylistRepository`**
55. `it_should_round_trip_the_channels_categorization_setting`.
56. `it_should_round_trip_the_playlists_categorization_setting`.

**`sqlite_migrations`**
57. `it_should_default_existing_sources_and_categories_after_the_ai_categorization_migration`: `false` and `NULL`.
