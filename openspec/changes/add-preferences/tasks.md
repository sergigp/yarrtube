# Tasks

## 1. Walking skeleton

- [x] 1.1 Create every backend and web file, type and signature from design.md, wired end-to-end: migration 0017 registered, `Preferences` (+ `Default`, `shows_home_categories` returning `true`), errors, `PreferencesRepository` + `SqlitePreferencesRepository` (`find` → `None`, `save` → `Ok(())`), `PreferencesSearcher`/`PreferencesUpdater` (return defaults / the input), `HomeLimits::following` (returns `self`), `VideoSearcher` taking the repository (unused), DTOs, `MISSING_*` messages, handlers (GET returns defaults, PUT echoes defaults), `/preferences` route, container/serve wiring; web: `Preferences` type, client + query stubs, `aPreferences` builder, `lib/preferences.ts` (both helpers return `true`), `ui/switch.tsx`, empty `PreferencesView` at `/preferences`. Add `'GET /api/preferences': aPreferences()` to existing tests rendering components that will read preferences. Done when `cargo build` succeeds, `cargo test --locked` passes and `npm run check` passes in `web/`; no new tests, no visible change.

## 2. Behaviour (TDD)

Backend acceptance tests (HTTP handlers):

- [x] 2.1 `preferences::it_should_return_the_defaults_if_nothing_saved` — all `true`; verify `cargo test preferences` passes
- [x] 2.2 `preferences::it_should_save_preferences` — PUT with `categories_enabled: false` → 200 with it, repository `find()` returns it
- [x] 2.3 `preferences::it_should_replace_previously_saved_preferences` — the second save wins
- [x] 2.4 `preferences::it_should_fail_to_save_if_a_preference_missing` — no `home_show_latest` → 400 with its message, nothing stored
- [x] 2.5 `preferences::it_should_return_saved_preferences` — seeded through the repository, GET returns them
- [x] 2.6 `videos::it_should_leave_out_continue_watching_if_turned_off` — a started short video appears under quick watches; verify `cargo test videos` passes
- [x] 2.7 `videos::it_should_leave_out_category_sections_if_categories_off` — no sections, Chess's videos under quick watches/latest
- [x] 2.8 `videos::it_should_leave_out_category_sections_if_home_categories_off`
- [x] 2.9 `videos::it_should_leave_out_quick_watches_if_turned_off` — the short video appears under latest
- [x] 2.10 `videos::it_should_leave_out_latest_if_turned_off`

Web (Vitest):

- [x] 2.11 `lib/preferences` › `categoriesShown`: loading → false, failed → true, loaded → `categories_enabled`; `homeTintShown` needs both flags; verify `npm run test` passes
- [x] 2.12 `SettingsMenu` › lists `Tasks`, `Preferences`, `Report a bug`; Preferences navigates to `/preferences`; Report a bug links to `https://github.com/sergigp/yarrtube/issues/new` in a new tab
- [x] 2.13 `AppShell` › `/preferences` renders the Preferences view
- [x] 2.14 `PreferencesView` › shows a switch per preference with its current value, grouped "Categories" and "Home"
- [ ] 2.15 `PreferencesView` › toggling a switch PUTs the full set with the change and the switch reflects the saved value
- [ ] 2.16 `PreferencesView` › a failed save shows the error and the switch returns to its saved value
- [ ] 2.17 `PreferencesView` › "Show categories" and "Show category colors" are disabled while "Enable categories" is off
- [ ] 2.18 `Sidebar` › no Categories section or "Add category" while categories are off, search labelled "Search channels and playlists"
- [ ] 2.19 `AddChannelDialog` › no Categories field while off, saving makes no category request
- [ ] 2.20 `AddPlaylistDialog` › same as 2.19
- [ ] 2.21 `EditChannelDialog` › same as 2.19
- [ ] 2.22 `EditPlaylistDialog` › same as 2.19
- [ ] 2.23 `CategoryDetail` › says "Categories are turned off." with a link to Preferences while off
- [ ] 2.24 `Home` › cards untinted and unframed when `home_show_category_colors` is off, and when categories are off
- [ ] 2.25 `Home` › no "Latest videos" when `home_show_latest` is off
- [ ] 2.26 `Home` › "Nothing to show here." with a Preferences link when latest is off and nothing else shows

## 3. Infrastructure adapters (TDD)

SqlitePreferencesRepository:

- [ ] 3.1 `it_should_find_nothing_if_never_saved`; verify `cargo test sqlite_preferences_repository` passes
- [ ] 3.2 `it_should_find_saved_preferences`
- [ ] 3.3 `it_should_read_missing_keys_as_defaults` — row `{"categories_enabled": false}` → every other preference `true`

## 4. Verification

- [ ] 4.1 `cargo test --locked`, `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` pass
- [ ] 4.2 `npm run check` in `web/` passes
- [ ] 4.3 Add `e2e-tests/tests/preferences.spec.js` (open Preferences from the header menu, turn categories off → sidebar Categories section gone and add channel dialog without Categories field, turn back on → back); `scripts/run-tests.sh` passes and the route coverage reporter lists no missed `/api/preferences` route
- [ ] 4.4 Manual (agent): start `scripts/run-local.sh --port <free port> --fresh-videos`, seed a playlist and a category with it via the HTTP API, then with the Playwright MCP browser: open Preferences from the gear menu, toggle each home switch and check the home sections and card tints change, turn categories off and check sidebar, add/edit dialogs, home and a category address, reload and check values persist (screenshot), check "Report a bug" has the issue URL with `target="_blank"`. Stop the server and delete `yarrtube.sqlite3` and the fresh videos dir.
- [ ] 4.5 Manual (human): judge the Preferences view's look (switch styling, grouping, dark mode) and on mobile width.

## Workflow follow-up

- Archive the change so `openspec/specs/preferences`, `video-listing` and `web-ui` are updated, then verify with `openspec validate --specs`.
