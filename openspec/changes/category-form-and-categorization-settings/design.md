## Files

**Created**
- `web/src/components/CategoryDialog.tsx` (+ `.test.tsx`): `CategorySettingsDialog` renamed; one form for create and edit. Absorbs `AddCategoryDialog.test.tsx` cases.
- `web/src/components/CategoryPromptHelp.tsx` (+ `.test.tsx`): the "?" popover and its guide, like `PromptHelp`.
- `web/src/lib/categoryForm.ts` (+ `.test.ts`): pure create/edit logic: what changed, whether it's dirty.

**Deleted**
- `web/src/components/AddCategoryDialog.tsx` (+ `.test.tsx`), `web/src/components/CategorySettingsDialog.tsx` (+ `.test.tsx`).

**Changed**
- `src/domain/preferences/categorization_prompt.rs`: `SHIPPED` points to `v2.md`.
- `src/domain/preferences/categorization_settings.rs`: three fields, hand-written `Default`.
- `src/domain/preferences/{context_length,temperature}.rs`: `categorization_default()`.
- `src/domain/preferences/{preferences,preferences_change}.rs`: apply the new fields.
- `src/domain/category/errors.rs`: `CreateCategoryError::MemberNotTracked`.
- `src/domain/services/category_creator.rs`: `NewCategory`, tracked check, reassignment.
- `src/application/http/categories/{mod,dto}.rs`: create takes prompt and members; `CategoryResponse` returns the real member lists.
- `src/application/http/preferences/{mod,dto}.rs`: new fields; test at `mod.rs:238` reads `v2.md`.
- `src/infrastructure/repositories/sqlite_preferences_repository.rs`: persist and read the new fields.
- `src/serve.rs`: `CategoryCreator::new` dependencies.
- `web/src/api/{types,client}.ts`, `web/src/lib/aiSettingsForm.ts`, `web/src/test/helpers.tsx` (`v2.md`): DTO mirror, form fields.
- `web/src/components/AiSettingsView.tsx`: Advanced fields, `categorizationCannotThink`, shared `ThinkingField`.
- `web/src/components/CategoryDetail.tsx`: removes `CategoryColorDialog`, `draftColor`, "Change color"; uses `CategoryDialog`.
- `web/src/App.tsx`: `CategoryDialog` for "Add category".
- `e2e-tests/tests/categories.spec.js`, `e2e-tests/tests/aiSettings.spec.js`: one-step create, new fields, exact labels.
- `doc/AI.md`, `doc/CATEGORIES.md`, `prompts/categorization/README.md`: v2 shipped, new settings, one dialog.

## Types & Signatures

```rust
// domain/preferences
pub struct CategorizationSettings {
    pub enabled: bool,
    pub model: CategorizationModel,
    pub context_length: ContextLength,
    pub temperature: Temperature,
    pub thinking: bool,
    /// The prompt the user saved; `None` uses the shipped one.
    pub prompt: Option<CategorizationPrompt>,
}
impl Default for CategorizationSettings {}          // 4096, 0, thinking on: the eval's settings

impl ContextLength { pub fn categorization_default() -> Self; }   // 4096
impl Temperature   { pub fn categorization_default() -> Self; }   // 0

pub struct PreferencesChange {
    /* … */
    pub categorization_context_length: Option<ContextLength>,
    pub categorization_temperature: Option<Temperature>,
    pub categorization_thinking: Option<bool>,
}

// domain/category/errors.rs
pub enum CreateCategoryError {
    NameTaken(CategoryName),
    MemberNotTracked(CategoryMember),
    Repository(anyhow::Error),
}

// domain/services/category_creator.rs
pub struct NewCategory {
    pub name: CategoryName,
    /// `None` takes the least used palette color.
    pub color: Option<CategoryColor>,
    pub ai_prompt: Option<CategoryPrompt>,
    pub members: Vec<CategoryMember>,
}

impl CategoryCreator {
    pub fn new(
        repository: Arc<dyn CategoryRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_category_assigner: VideoCategoryAssigner,
        clock: Arc<dyn Clock>,
    ) -> Self;
}
pub trait CategoryCreatorApi: Send + Sync {
    fn create(&self, category: NewCategory) -> Result<Category, CreateCategoryError>;
}

// application/http/categories/dto.rs
pub struct CreateCategoryRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub ai_prompt: Option<String>,
    pub channels: Vec<String>,     // #[serde(default)]
    pub playlists: Vec<String>,    // #[serde(default)]
}

// application/http/categories/mod.rs: shared with set_category_members
fn members_from(channels: Vec<String>, playlists: Vec<String>) -> Result<Vec<CategoryMember>, ApiError>;
```

`CategoryRepository::insert` already stores the prompt and members in one transaction: no port change.

`CategorizationSettings` must not take the `ContextLength`/`Temperature` `Default`s: those are the summary values. The repository's fallback for a missing or invalid stored value reads `defaults.categorization`.

```ts
// web/src/api/types.ts
interface Preferences {
  /* … */
  categorization_context_length: number
  categorization_temperature: number
  categorization_thinking: boolean
}
interface NewCategory extends CategoryMembers { name: string; color: string; ai_prompt?: string }

// web/src/api/client.ts
function createCategory(category: NewCategory): Promise<CreatedCategory>

// web/src/lib/aiSettingsForm.ts
function changedFields(
  saved: Preferences,
  form: AiSettingsForm,
  options?: { summariesThinking: boolean; categorizationThinking: boolean },
): PreferencesPatch

// web/src/lib/categoryForm.ts
interface CategoryFields { name: string; color: string; prompt: string }
/** The fields of `fields` that differ from `base`, as an update; prompt only when `withPrompt`. */
function changedDetails(
  base: CategoryFields,
  fields: CategoryFields,
  { withPrompt }: { withPrompt: boolean },
): { name?: string; color?: string; ai_prompt?: string }

// web/src/components/CategoryDialog.tsx
interface CategoryDialogProps {
  /** Absent when creating. */
  category?: Category
  open: boolean
  onOpenChange: (open: boolean) => void
}

// web/src/components/CategoryPromptHelp.tsx
function CategoryPromptHelp(): JSX.Element
```

The dialog body is a `<form>`: Enter in the search field must always `preventDefault`, not only on a single match. Height goes from `40rem` to `48rem`.

## Call Stack

**Create**
```
CategoryDialog.save()
  createCategory({ name, color, ai_prompt?, channels, playlists })   // ai_prompt only while the field shows and isn't blank
    POST /api/categories
      create_category(State<CategoryCreator>, Json<CreateCategoryRequest>)
        CategoryName::new, CategoryColor::new, CategoryPrompt::new, members_from
        CategoryCreator::create(NewCategory)
          video_category_assigner.lock()
          repository.list() -> ensure_name_free(existing, name)
          ensure_tracked(members) -> channel_repository.find / playlist_repository.find
          Category::create(name, color, now).with_ai_prompt(..).with_members(..)
          repository.insert(category)                                 // one transaction
          repository.list() -> categories                             // re-read: includes the new one, like set_members
          video_category_assigner.reassign_category(id, &[], categories)
  invalidateLibrary(); onClose(); navigate(`/categories/${id}`)
```

**Edit**
```
CategoryDialog.save()
  changedDetails(base, fields, { withPrompt })
  updateCategory(id, details)            // when not empty: PATCH, one save; 409 stops here
  setBase(fields)
  setCategoryMembers(id, staged)         // when sources changed: PUT
  onClose()
```

**Preferences**
```
PATCH /api/preferences
  update_preferences -> ContextLength::new, Temperature::new -> PreferencesChange
    Preferences::apply -> CategorizationSettings { context_length, temperature, thinking, .. }
GET /api/preferences -> PreferencesResponse::from(Preferences)
```

## Test Plan

1. Behaviour tests

`http/preferences`
1. `it_should_ship_the_categorization_prompt_v2`: `categorization_prompt` reads `v2.md`, `is_default` true.
2. `it_should_return_the_categorization_generation_defaults_if_nothing_saved`: 4096, 0, thinking on.
3. `it_should_save_the_categorization_generation_settings`: returned and stored; summary values unchanged.
4. `it_should_fail_if_the_categorization_context_length_is_out_of_range`: 400 with its message.
5. `it_should_fail_if_the_categorization_temperature_is_out_of_range`: 400 with its message.

`http/categories`

6. `it_should_create_a_category_with_a_prompt`: response and stored prompt, trimmed.
7. `it_should_create_a_category_with_members`: response holds the lists, stored members.
8. `it_should_put_the_videos_of_its_members_in_a_created_category`: stored videos belong to it, events announced.
9. `it_should_create_a_category_without_a_prompt_if_blank`: `""` stores `None`.
10. `it_should_fail_to_create_a_category_with_an_untracked_member`: 400, nothing stored.
11. `it_should_fail_to_create_a_category_if_the_prompt_is_too_long`: 400, nothing stored.

Web

12. `aiSettingsForm.test.ts`: `changedFields` includes the new fields only when changed; drops `categorization_thinking` when that model can't think; `hasInvalidNumber` covers both new numbers.
13. `AiSettingsView.test.tsx`: categorization Advanced shows `4096`, `0`, thinking on and the prompt.
14. `AiSettingsView.test.tsx`: saves only the changed advanced field, independently of summaries.
15. `AiSettingsView.test.tsx`: a model that can't think hides only that section's switch.
16. `AiSettingsView.test.tsx`: an emptied categorization number blocks saving with the message.
17. `categoryForm.test.ts`: `changedDetails` returns only what differs; no prompt without `withPrompt`.
18. `CategoryDialog.test.tsx`: create shows name, color, prompt, chips in order; name focused; next free color preselected.
19. `CategoryDialog.test.tsx`: create sends everything in one `POST` and navigates with no dialog open.
20. `CategoryDialog.test.tsx`: create without categorization has no prompt field and sends no `ai_prompt`.
21. `CategoryDialog.test.tsx`: create failing keeps everything and shows the error; blank name disables it.
22. `CategoryDialog.test.tsx`: create dismissal asks to discard only with something entered.
23. `CategoryDialog.test.tsx`: edit prefills name and color; saving sends one update and no members.
24. `CategoryDialog.test.tsx`: edit with a taken name keeps the dialog and doesn't send members.
25. `CategoryDialog.test.tsx`: the new partial failure and discard messages.
26. `CategoryDialog.test.tsx`: Enter in the search with several matches neither toggles nor saves; Enter in "Name" saves.
27. `CategoryPromptHelp.test.tsx`: opening shows the good and bad examples and tips.
28. `CategoryDialog.test.tsx`: closing the guide with Escape keeps the dialog and the staged change.
29. `CategoryDetail.test.tsx`: no "Change color" action; the color changes through "Edit settings".

2. Infrastructure tests

`sqlite_preferences_repository`

30. `it_should_read_the_categorization_defaults_if_stored_preferences_predate_them`: 4096, 0, true, not the summary values.
31. `it_should_read_invalid_categorization_settings_as_their_defaults`: out-of-range stored values fall back to 4096 and 0.
