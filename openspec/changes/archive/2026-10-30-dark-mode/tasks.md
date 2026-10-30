## 1. Walking skeleton

- [x] 1.1 Add `Theme`, `Preferences.theme`, the DTO fields, the stored JSON key, the frontend `theme` field, `useTheme`/`useSavedTheme`/`applyTheme`, `useSaveTheme` and `ThemeToggle` in the header, with fixtures and e2e defaults updated; verify `cargo build` and the existing Rust and web tests pass

## 2. Behaviour (TDD)

- [x] 2.1 `it_should_return_the_defaults_if_nothing_saved` includes `theme: "light"`; verify with `cargo test preferences`
- [x] 2.2 `it_should_save_the_theme`: saving `"dark"` returns and stores `Theme::Dark`; verify with `cargo test preferences`
- [x] 2.3 `it_should_fail_to_save_if_theme_missing`: 400 with the missing-theme message; verify with `cargo test preferences`
- [x] 2.4 `it_should_fail_to_save_if_theme_invalid`: 400 with the unknown-theme message; verify with `cargo test preferences`
- [x] 2.5 `Theme` value object tests (light, dark, unknown); verify with `cargo test theme`
- [x] 2.6 `ThemeToggle` tests: dark on and saved, back to light, revert on failed save, disabled while loading; verify with `npm run test -- ThemeToggle`
- [x] 2.7 `AppShell` tests: saved dark theme drawn, saved light overrides cached dark; verify with `npm run test -- AppShell`
- [x] 2.8 `categoryColors` dark tint and ink tests (hue kept, dark, at least 4.5:1); category chips, picker and header pass the drawn theme; verify with `npm run test -- categoryColors`
- [x] 2.9 Dark palette tokens under `.dark`, the first-paint script in `index.html`, straight video list separators, and theme-aware swatch borders, warning icon and dialog backdrop; verify with `npm run check`

## 3. Infrastructure adapters (TDD)

- [x] 3.1 `SqlitePreferencesRepository::it_should_find_saved_preferences` round-trips `Theme::Dark`; verify with `cargo test sqlite_preferences`
- [x] 3.2 `SqlitePreferencesRepository::it_should_read_missing_keys_as_defaults` reads a missing `theme` as `Theme::Light`; verify with `cargo test sqlite_preferences`

## 4. Verification

- [x] 4.1 Run `scripts/run-tests.sh` with no flags (web, Rust fmt/clippy/tests, contract, e2e including the new dark mode spec) and verify every suite passes
- [x] 4.2 Manual (agent): start `scripts/run-local.sh --port 8099 --fresh-videos`, use the Playwright MCP browser to flip the header switch, and confirm `<html>` gets `dark`, the switch shows the moon on a green track, and a reload stays dark; then stop the server and delete `yarrtube.sqlite3` and the fresh videos dir
- [ ] 4.3 Manual (human): with real videos, check by eye that dark mode looks right on the home view, a channel view's video list (straight separators), a category view header and chips, dialogs and dropdown menus, and on a phone (header switch fits, browser bar color is dark)
