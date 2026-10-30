## Files

**Backend**
- `src/domain/preferences/theme.rs` (new): `Theme` value object, the only valid theme strings.
- `src/domain/preferences/preferences.rs`: `Preferences` gains `theme`, defaulting to `Theme::Light`.
- `src/domain/preferences/mod.rs`: declares and re-exports `theme`.
- `src/application/http/preferences/dto.rs`: `theme` on the request (`Option<String>`) and response (`String`).
- `src/application/http/preferences/mod.rs`: `update_preferences` requires `theme` and builds a `Theme` from it; new missing-theme message.
- `src/infrastructure/repositories/sqlite_preferences_repository.rs`: `theme` key in the stored JSON document; missing or unknown reads as the default. No migration, since preferences are one JSON column.

**Web UI**
- `web/index.html`: inline script that adds `dark` to `<html>` before first paint when the cached theme is dark.
- `web/src/index.css`: `dark` variant bound to the `.dark` class; dark values for every color token under `.dark`.
- `web/src/lib/theme.ts` (new): `Theme` type and the cache key.
- `web/src/hooks/useTheme.ts` (new): reads the drawn theme, draws and caches one, and draws the saved preference when it loads.
- `web/src/components/ThemeToggle.tsx` (new): the header switch.
- `web/src/App.tsx`: renders `ThemeToggle` before `SettingsMenu`; calls `useSavedTheme()`.
- `web/src/api/types.ts`, `web/src/lib/preferences.ts`, `web/src/test/helpers.tsx`: `theme` on `Preferences`, its default and the `aPreferences` builder.
- `web/src/api/queries.ts`: `useSaveTheme`, which saves without refetching home videos.
- `web/src/components/PreferencesView.tsx`: its switch keys exclude `theme`.
- `web/src/lib/categoryColors.ts`: per-theme soft tint and ink.
- `web/src/components/CategoryChips.tsx`, `CategoryPicker.tsx`, `CategoryDetail.tsx`: pass the drawn theme to the tint helpers.
- `web/src/components/VideoListPane.tsx`: straight `::after` separators instead of `divide-y` on rounded rows.
- `web/src/components/SidebarCategories.tsx`, `CategoryColorPicker.tsx`, `CategoryPicker.tsx`, `VideoListPane.tsx`, `ui/dialog.tsx`: hard-coded black/amber colors made to work in both themes.
- `web/src/test/setup.ts`: removes `dark` from `<html>` after each test.

**E2E**
- `e2e-tests/helpers/fixtures.js`: default preferences include `theme: 'light'`.
- `e2e-tests/tests/preferences.spec.js`: the saved theme survives clearing browser storage.

## Types & Signatures

```rust
// src/domain/preferences/theme.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme { #[default] Light, Dark }

impl Theme {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError>;
    pub fn as_str(&self) -> &'static str;
}

// src/domain/preferences/preferences.rs
pub struct Preferences {
    // ...existing booleans
    pub theme: Theme,
}

// src/application/http/preferences/dto.rs
pub struct UpdatePreferencesRequest { /* ... */ #[serde(default)] pub theme: Option<String> }
pub struct PreferencesResponse { /* ... */ pub theme: String }
```

```ts
// web/src/api/types.ts
interface Preferences { /* ... */ theme: 'light' | 'dark' }

// web/src/lib/theme.ts
type Theme = Preferences['theme']
const THEME_KEY = 'yarrtube.theme'

// web/src/hooks/useTheme.ts
function useTheme(): Theme
function useSavedTheme(): void
function applyTheme(theme: Theme): void

// web/src/api/queries.ts
function useSaveTheme(): (preferences: Preferences, theme: Preferences['theme']) => Promise<void>

// web/src/lib/categoryColors.ts
function softTint(hex: string, theme?: Theme): string // default 'light'
function inkShade(hex: string, theme?: Theme): string // default 'light'
```

## Call Stack

**First paint**
1. `index.html` script → `localStorage.getItem('yarrtube.theme')` → `'dark'` adds `dark` to `<html>` and sets `meta[name=theme-color]`.

**Saved theme loads**
1. `AppShell` → `useSavedTheme()` → `usePreferences()` (`GET /api/preferences`).
2. When `data.theme` changes → `applyTheme(data.theme)` → toggles the `dark` class, sets theme-color, caches it, notifies `useTheme` subscribers.

**Flipping the switch**
1. `ThemeToggle` `onCheckedChange(checked)` → `applyTheme(next)`.
2. → `useSaveTheme()(preferences, next)` → `savePreferences({ ...preferences, theme: next })` (`PUT /api/preferences`).
3. Backend: `update_preferences(State<PreferencesUpdater>, Json(request))` → `Theme::new(required(request.theme, MISSING_THEME)?)?` → `PreferencesUpdater::save(preferences)` → `SqlitePreferencesRepository::save(&preferences)`.
4. On success → `queryClient.setQueryData(preferences, saved)`; on failure → `applyTheme(preferences.theme)`.

**Category tint**
1. `CategoryChips` / `CategoryPicker` / `CategoryDetail` → `useTheme()` → `softTint(color, theme)`, `inkShade(color, theme)`.

## Test Plan

1. Behaviour tests: HTTP handler (`src/application/http/preferences/mod.rs`)
   - `it_should_return_the_defaults_if_nothing_saved`: the response includes `theme: "light"`.
   - `it_should_save_the_theme`: `theme: "dark"` is returned and stored as `Theme::Dark`.
   - `it_should_fail_to_save_if_theme_missing`: 400 `Theme must be stated (missing)`.
   - `it_should_fail_to_save_if_theme_invalid`: 400 `Theme must be "light" or "dark" (got "sepia")`.
2. Behaviour tests: web UI
   - `ThemeToggle`: switches to dark and PUTs every preference with `theme: 'dark'`; switches back to light; draws the previous theme back when the save fails; is disabled while preferences load.
   - `AppShell`: draws the saved dark theme once preferences load; the saved light theme overrides a cached dark one.
   - `categoryColors`: the dark tint keeps the hue and is dark; the dark ink on the dark tint is at least 4.5:1.
3. Value object tests: `Theme` (`theme.rs`)
   - `it_should_parse_light`, `it_should_parse_dark`, `it_should_reject_an_unknown_theme`.
4. Infrastructure tests: `SqlitePreferencesRepository`
   - `it_should_find_saved_preferences`: round-trips `theme: Theme::Dark`.
   - `it_should_read_missing_keys_as_defaults`: a document without `theme` reads as `Theme::Light`.
5. E2E: `preferences: the dark mode switch is saved on the daemon, not just in the browser`.
