# Design

Storage: one row (`id = 1`) holding the preferences as a JSON document. Keys missing from the stored JSON read as `Preferences::default()`, so adding a preference later needs no migration. Typed columns (a migration per preference) and a config file next to the DB (a second persistence mechanism, not covered by backups of the DB) were rejected.

## Files

- `migrations/0017_preferences.sql` (new): `preferences (id INTEGER PRIMARY KEY CHECK (id = 1), data TEXT NOT NULL)`.
- `src/infrastructure/shared/sqlite_migrations.rs`: registers 0017.
- `src/domain/preferences/preferences.rs` (new): the `Preferences` entity, its `Default` (all `true`) and the derived rules (`shows_home_categories`).
- `src/domain/preferences/errors.rs` (new): `ReadPreferencesError`, `SavePreferencesError`.
- `src/domain/preferences/mod.rs` (new), `src/domain/mod.rs`: module wiring.
- `src/domain/services/preferences_searcher.rs` (new): reads them, falling back to the defaults.
- `src/domain/services/preferences_updater.rs` (new): saves them.
- `src/domain/services/mod.rs`: exports both.
- `src/domain/video/home_limits.rs`: `following(&Preferences)` zeroes the limits of sections turned off.
- `src/domain/services/video_searcher.rs`: takes a `PreferencesRepository`; `list_home` applies `limits.following(..)`.
- `src/infrastructure/repositories/sqlite_preferences_repository.rs` (new): `PreferencesRepository` trait + SQLite impl; private serde `StoredPreferences` with optional fields.
- `src/infrastructure/repositories/mod.rs`: registers it.
- `src/infrastructure/infrastructure_container.rs`, `src/serve.rs`: build the repository and services, inject into `VideoSearcher` and `ApiServices`.
- `src/application/http/preferences/mod.rs` (new): `get_preferences`, `update_preferences`, one `MISSING_*` message per preference (local, like `categories`) + acceptance tests.
- `src/application/http/preferences/dto.rs` (new): `PreferencesResponse`, `UpdatePreferencesRequest`.
- `src/application/http/mod.rs`: `ApiServices` fields, `/preferences` route (GET, PUT).
- `src/application/http/videos/mod.rs`: existing `VideoSearcher::new` calls get the repository; new home acceptance tests.
- `web/src/api/types.ts`, `web/src/api/client.ts`, `web/src/api/queries.ts`: `Preferences`, `fetchPreferences`/`savePreferences`, `usePreferences`/`useSavePreferences`/`useCategoriesShown`.
- `web/src/lib/preferences.ts` (+ test, new): `DEFAULT_PREFERENCES`, `categoriesShown`, `homeTintShown`.
- `web/src/test/helpers.tsx`: `aPreferences` builder.
- `web/src/components/ui/switch.tsx` (new): shadcn switch on `radix-ui`.
- `web/src/components/PreferencesView.tsx` (+ test, new): the view.
- `web/src/components/SettingsMenu.tsx` (+ test): "Preferences" and "Report a bug".
- `web/src/App.tsx` (+ `AppShell.test.tsx`): `/preferences` route.
- `web/src/components/Home.tsx` (+ test): tint gate, latest gate, "Nothing to show here."
- `web/src/components/Sidebar.tsx` (+ test): Categories section and search label gated.
- `web/src/components/AddChannelDialog.tsx`, `AddPlaylistDialog.tsx`, `EditChannelDialog.tsx`, `EditPlaylistDialog.tsx` (+ tests): Categories field gated; no category save when hidden.
- `web/src/components/CategoryDetail.tsx` (+ test): "Categories are turned off." when off.
- Existing web tests rendering any gated component: add `'GET /api/preferences': aPreferences()` to their route tables.
- `e2e-tests/tests/preferences.spec.js` (new): toggles categories off/on through the UI (covers GET and PUT routes).

## Types & Signatures

```rust
// domain/preferences/preferences.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preferences {
    pub categories_enabled: bool,
    pub home_show_continue_watching: bool,
    pub home_show_categories: bool,
    pub home_show_category_colors: bool,
    pub home_show_quick_watches: bool,
    pub home_show_latest: bool,
}
impl Default for Preferences { /* all true */ }
impl Preferences {
    /// Category sections on home need categories on as well.
    pub fn shows_home_categories(&self) -> bool;
}

// domain/preferences/errors.rs
pub enum ReadPreferencesError { Repository(anyhow::Error) }
pub enum SavePreferencesError { Repository(anyhow::Error) }

// infrastructure/repositories/sqlite_preferences_repository.rs
pub trait PreferencesRepository: Send + Sync {
    fn find(&self) -> anyhow::Result<Option<Preferences>>;
    /// Inserts or replaces the single row.
    fn save(&self, preferences: &Preferences) -> anyhow::Result<()>;
}
pub struct SqlitePreferencesRepository { db: Database }
#[derive(Serialize, Deserialize)]
struct StoredPreferences { categories_enabled: Option<bool>, /* … one Option<bool> per field */ }
// None -> the field of Preferences::default()

// domain/services/preferences_searcher.rs
pub trait PreferencesSearcherApi: Send + Sync {
    /// The saved preferences, or the defaults when none were saved.
    fn get(&self) -> Result<Preferences, ReadPreferencesError>;
}
// domain/services/preferences_updater.rs
pub trait PreferencesUpdaterApi: Send + Sync {
    fn save(&self, preferences: Preferences) -> Result<Preferences, SavePreferencesError>;
}

// domain/video/home_limits.rs
impl HomeLimits {
    /// 0 for every section the preferences turn off.
    pub fn following(self, preferences: &Preferences) -> Self;
}

// domain/services/video_searcher.rs
VideoSearcher::new(/* existing … */, preferences_repository: Arc<dyn PreferencesRepository>)

// application/http/preferences/dto.rs
#[derive(Deserialize)]
pub struct UpdatePreferencesRequest { pub categories_enabled: Option<bool>, /* … all Option<bool> */ }
#[derive(Serialize, PartialEq, Debug)]
pub struct PreferencesResponse { pub categories_enabled: bool, /* … all bool */ }
impl From<Preferences> for PreferencesResponse;

// application/http/preferences/mod.rs
pub async fn get_preferences(State<PreferencesSearcher>) -> Result<Json<PreferencesResponse>, ApiError>;
pub async fn update_preferences(State<PreferencesUpdater>, Json<UpdatePreferencesRequest>)
    -> Result<Json<PreferencesResponse>, ApiError>;
```

```ts
// web/src/api/types.ts
export interface Preferences {
  categories_enabled: boolean
  home_show_continue_watching: boolean
  home_show_categories: boolean
  home_show_category_colors: boolean
  home_show_quick_watches: boolean
  home_show_latest: boolean
}
// web/src/api/client.ts
export function fetchPreferences(): Promise<Preferences>
export function savePreferences(preferences: Preferences): Promise<Preferences>
// web/src/api/queries.ts
export function usePreferences(): UseQueryResult<Preferences, Error>       // key ['preferences']
export function useSavePreferences(): (preferences: Preferences) => Promise<void>
export function useCategoriesShown(): boolean // categoriesShown(data, isError)
// sets ['preferences'] to the response, invalidates home videos
// web/src/lib/preferences.ts
export const DEFAULT_PREFERENCES: Preferences
/** Hidden while loading; the defaults when loading failed. */
export function categoriesShown(preferences: Preferences | undefined, failed: boolean): boolean
export function homeTintShown(preferences: Preferences | undefined, failed: boolean): boolean
```

## Call Stack

Read: `GET /api/preferences` → `get_preferences` → `PreferencesSearcher::get()` → `PreferencesRepository::find()` → `unwrap_or_default()` → 200 `PreferencesResponse`.

Save: `PUT /api/preferences {…}` → `update_preferences` → `required(field, MISSING_X)?` per field → `Preferences { … }` → `PreferencesUpdater::save(preferences)` → `PreferencesRepository::save(&preferences)` → 200 `PreferencesResponse`.

Home: `GET /api/videos/home` → `list_home_videos` → `VideoSearcher::list_home(HOME_LIMITS)` → `preferences_repository.find()?.unwrap_or_default()` → `limits.following(&preferences)` → sections as today (a 0 limit picks no video, so the videos flow down).

Web toggle: `PreferencesView` switch → `useSavePreferences()({ ...current, [key]: value })` → `PUT /api/preferences` → cache `['preferences']` updated → every `usePreferences()` consumer re-renders; home refetched. On error: message shown, switch reverts (cache untouched).

Web gating: `Sidebar` / dialogs → `useCategoriesShown()`; `CategoryDetail` → `usePreferences()` (says "turned off" only once loaded and off); `Home` → `homeTintShown(..)` decides whether `categoryColors` is passed to `VideoGrid`, `home_show_latest` decides the "Latest videos" section.

Menu: `SettingsMenu` → `Link to="/preferences"`; `<a href="https://github.com/sergigp/yarrtube/issues/new" target="_blank" rel="noreferrer">`.

## Test Plan

1. Behaviour tests (HTTP handlers)
   - `preferences::it_should_return_the_defaults_if_nothing_saved`: all `true`.
   - `preferences::it_should_save_preferences`: PUT with `categories_enabled: false` → 200 with it; `PreferencesRepository::find()` returns it.
   - `preferences::it_should_replace_previously_saved_preferences`: saved then saved again → the second set.
   - `preferences::it_should_fail_to_save_if_a_preference_missing`: no `home_show_latest` → 400 with its message, nothing stored (unmigrated connection).
   - `preferences::it_should_return_saved_preferences`: seeded via repository → GET returns them.
   - `videos::it_should_leave_out_continue_watching_if_turned_off`: started short video → in quick watches.
   - `videos::it_should_leave_out_category_sections_if_categories_off`: "Chess" pending videos → no sections, videos under quick watches/latest.
   - `videos::it_should_leave_out_category_sections_if_home_categories_off`
   - `videos::it_should_leave_out_quick_watches_if_turned_off`: short video → under latest.
   - `videos::it_should_leave_out_latest_if_turned_off`
2. Infrastructure tests
   - SqlitePreferencesRepository
     - `it_should_find_nothing_if_never_saved`
     - `it_should_find_saved_preferences`
     - `it_should_read_missing_keys_as_defaults`: row with `{"categories_enabled": false}` → others `true`.
3. Web (Vitest)
   - `lib/preferences`: `categoriesShown` (loading → false, failed → true, loaded → value); `homeTintShown` needs both flags.
   - `SettingsMenu`: items `Tasks`, `Preferences`, `Report a bug`; Preferences navigates; Report a bug has the issue URL and `target="_blank"`.
   - `PreferencesView`: shows current values; toggling PUTs the full set with the change; save error shows message and reverts; category home switches disabled while categories off.
   - `Sidebar`: no Categories section/Add category while off; search label "Search channels and playlists".
   - `AddChannelDialog` / `AddPlaylistDialog` / `EditChannelDialog` / `EditPlaylistDialog`: no Categories field while off, no category request on save.
   - `CategoryDetail`: "Categories are turned off." with a Preferences link.
   - `Home`: cards untinted with colors off; untinted with categories off; no "Latest videos" when off; "Nothing to show here." with a Preferences link.
   - `AppShell`: `/preferences` renders the view.
