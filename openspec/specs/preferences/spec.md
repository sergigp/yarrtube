# preferences Specification

## Purpose
Holds the user's daemon-wide display preferences (whether categories are used, which home sections show), persisted by the daemon so every browser and device sees the same choices.

## Requirements

### Requirement: Preferences And Their Defaults
The daemon SHALL keep one set of preferences: `categories_enabled`, `home_show_continue_watching`, `home_show_categories`, `home_show_quick_watches` and `home_show_latest`, each a boolean that defaults to `true`, and `theme`, either `"light"` or `"dark"`, that defaults to `"light"`. A preference never saved SHALL read as its default, including after an upgrade that adds a new preference. A stored `theme` that is neither `"light"` nor `"dark"` SHALL read as `"light"`.

#### Scenario: Fresh install
- **WHEN** no preferences have been saved
- **THEN** every boolean preference reads as `true` and `theme` reads as `"light"`

#### Scenario: Preferences survive a restart
- **WHEN** `categories_enabled` is saved as `false` and the daemon restarts
- **THEN** `categories_enabled` reads as `false`

#### Scenario: Upgrading from a version without a theme
- **WHEN** preferences were saved by a version that had no `theme` preference
- **THEN** the saved preferences keep their values and `theme` reads as `"light"`

### Requirement: Read Preferences
The daemon SHALL provide `GET /api/preferences`, responding with HTTP status 200 and every preference with its current value.

#### Scenario: Reading the defaults
- **WHEN** a client requests `GET /api/preferences` and nothing has been saved
- **THEN** the daemon responds with HTTP status 200, every boolean preference set to `true` and `theme` set to `"light"`

### Requirement: Save Preferences
The daemon SHALL provide `PUT /api/preferences` taking every boolean preference as a boolean and `theme` as `"light"` or `"dark"`. On success it SHALL store them all, replacing the previous values, and respond with HTTP status 200 and the saved preferences. A request missing a preference SHALL be rejected with HTTP status 400, a `theme` other than `"light"` or `"dark"` SHALL be rejected with HTTP status 400 and the message `Theme must be "light" or "dark" (got "<value>")`, and a malformed body (e.g. a non-boolean value) as by every other endpoint; none SHALL change anything.

#### Scenario: Saving preferences
- **WHEN** a client sends `PUT /api/preferences` with `categories_enabled` `false`, every other boolean preference `true` and `theme` `"light"`
- **THEN** the daemon responds with HTTP status 200 and those values, and a later `GET /api/preferences` returns them

#### Scenario: Saving the dark theme
- **WHEN** a client sends `PUT /api/preferences` with every boolean preference `true` and `theme` `"dark"`
- **THEN** the daemon responds with HTTP status 200 and `theme` `"dark"`, and a later `GET /api/preferences` returns `theme` `"dark"`

#### Scenario: Missing preference
- **WHEN** a client sends `PUT /api/preferences` without `home_show_latest`
- **THEN** the daemon responds with HTTP status 400 and the stored preferences are unchanged

#### Scenario: Missing theme
- **WHEN** a client sends `PUT /api/preferences` without `theme`
- **THEN** the daemon responds with HTTP status 400 and the message `Theme must be stated (missing)`, and the stored preferences are unchanged

#### Scenario: Unknown theme
- **WHEN** a client sends `PUT /api/preferences` with `theme` `"sepia"`
- **THEN** the daemon responds with HTTP status 400 and the message `Theme must be "light" or "dark" (got "sepia")`, and the stored preferences are unchanged

### Requirement: Category Data Kept While Categories Are Off
Turning `categories_enabled` off SHALL NOT change any category, category membership, video category or Plex category label, and the category endpoints SHALL keep working, so turning it back on shows everything as before.

#### Scenario: Turning categories back on
- **WHEN** "Chess" has 2 sources, categories are turned off and then on again
- **THEN** "Chess" still exists with its 2 sources and its videos still belong to it
