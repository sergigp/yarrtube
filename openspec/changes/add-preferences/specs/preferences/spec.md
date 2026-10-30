# Spec Delta

## Purpose

Holds the user's daemon-wide display preferences (whether categories are used, which home sections show), persisted by the daemon so every browser and device sees the same choices.

## ADDED Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_category_colors`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved
- **THEN** every preference reads as `true`

#### Scenario: Preferences survive a restart
- **WHEN** `categories_enabled` is saved as `false` and the daemon restarts
- **THEN** `categories_enabled` reads as `false`

### Requirement: Read Preferences
The daemon SHALL provide `GET /api/preferences`, responding with HTTP status 200 and every preference with its current value.

#### Scenario: Reading the defaults
- **WHEN** a client requests `GET /api/preferences` and nothing has been saved
- **THEN** the daemon responds with HTTP status 200 and every preference set to `true`

### Requirement: Save Preferences
The daemon SHALL provide `PUT /api/preferences` taking every preference as a boolean. On success it SHALL store them all, replacing the previous values, and respond with HTTP status 200 and the saved preferences. A request missing a preference SHALL be rejected with HTTP status 400, and a malformed body (e.g. a non-boolean value) as by every other endpoint; neither SHALL change anything.

#### Scenario: Saving preferences
- **WHEN** a client sends `PUT /api/preferences` with `categories_enabled` `false` and every other preference `true`
- **THEN** the daemon responds with HTTP status 200 and those values, and a later `GET /api/preferences` returns them

#### Scenario: Missing preference
- **WHEN** a client sends `PUT /api/preferences` without `home_show_latest`
- **THEN** the daemon responds with HTTP status 400 and the stored preferences are unchanged

### Requirement: Category Data Kept While Categories Are Off
Turning `categories_enabled` off SHALL NOT change any category, category membership, video category or Plex category label, and the category endpoints SHALL keep working, so turning it back on shows everything as before.

#### Scenario: Turning categories back on
- **WHEN** "Chess" has 2 sources, categories are turned off and then on again
- **THEN** "Chess" still exists with its 2 sources and its videos still belong to it
