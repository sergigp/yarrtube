# Proposal

## Why

Categories and every home section are always on, so a user who doesn't use categories, or wants a quieter home, can't turn them off. These choices must follow the user across browsers and devices, so they belong in the daemon, not in `localStorage`.

## What Changes

- New daemon-wide preferences, stored in SQLite as one JSON document in a single-row `preferences` table (missing keys read as their default, so a new preference needs no migration):
  - `categories_enabled` (default on)
  - home: `home_show_continue_watching`, `home_show_categories`, `home_show_category_colors`, `home_show_quick_watches`, `home_show_latest` (all default on)
- `GET /api/preferences` returns them; `PUT /api/preferences` replaces them all.
- The home endpoint leaves out sections the preferences turn off (category sections also when categories are off), so their videos can fall to the sections below, as when a section is full.
- New "Preferences" view (`/preferences`): a switch per preference, saved as soon as it is toggled. The home switches for category sections and colors are disabled while categories are off.
- With categories off, the web UI shows no category UI anywhere: no sidebar Categories section, no Categories field in the add/edit channel and playlist dialogs, no home category sections, no card tints; a category view address says categories are turned off. Category data, video categories and Plex labels are untouched, so turning categories back on restores everything.
- With category colors off, home cards are untinted and unframed, like on a category view.
- The header settings menu gains "Preferences" (navigates to `/preferences`) and "Report a bug" (opens `https://github.com/sergigp/yarrtube/issues/new` in a new tab).

Assumptions (flag if wrong):
- "Show quick watches" is added alongside the four home toggles asked for, since it's the only other home section.
- Turning categories off is a display preference only: Plex category labels keep syncing.
- With every section off or empty, home says "Nothing to show here." with a link to Preferences.

## Capabilities

### New Capabilities
- `preferences`: reading and saving the daemon-wide user preferences (storage, defaults, HTTP API).

### Modified Capabilities
- `video-listing`: the home endpoint honours the home and category preferences.
- `web-ui`: Preferences view, header menu items, home sections/tints and all category UI gated by preferences.

## Impact

- Backend: migration `0017_preferences.sql`; new `domain/preferences` aggregate, `PreferencesSearcher`/`PreferencesUpdater` services, `SqlitePreferencesRepository`, `http/preferences` handlers; `VideoSearcher::list_home` reads preferences.
- Web: `api` types/client/query for preferences, `PreferencesView`, `SettingsMenu`, `Home`, `Sidebar`, add/edit channel and playlist dialogs, `CategoryDetail`, `App` route.
- e2e: a preferences spec exercising both new routes.
