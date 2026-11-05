## Purpose

Lets each tracked channel and playlist opt in to AI categorization, so that once videos are evaluated against categories it only happens for the sources the user chose.

## ADDED Requirements

### Requirement: Source Categorization Setting
Every stored channel and playlist SHALL have a `categorization_enabled` boolean, `false` unless set. Sources stored before this setting existed SHALL have `false`. The setting SHALL have no effect on syncing, downloading or a video's categories yet.

#### Scenario: Existing sources after upgrade
- **WHEN** the daemon starts on a database with channels and playlists stored before this setting existed
- **THEN** every one of them has `categorization_enabled` `false`

#### Scenario: Nothing is evaluated
- **WHEN** a channel with `categorization_enabled` `true` syncs a new video
- **THEN** the video's categories are the same as with `categorization_enabled` `false`

### Requirement: Categorization Needs AI Categorization On
A create or update request for a channel or playlist that supplies `categorization_enabled` `true` SHALL be rejected with a conflict response, changing nothing, unless the preferences have AI, categories and categorization all on. The error SHALL read `Turn on AI categorization before enabling it for a channel` (or `for a playlist`). On creation the check SHALL run before YouTube is looked up. `categorization_enabled` `false` SHALL always be accepted.

#### Scenario: Creating with categorization while AI is off
- **WHEN** AI is off and a channel creation request supplies `categorization_enabled` `true`
- **THEN** the system stores nothing, does not look the channel up on YouTube, and returns a conflict with `Turn on AI categorization before enabling it for a channel`

#### Scenario: Enabling while categorization is off
- **WHEN** AI is on, categorization is off and a playlist update supplies `categorization_enabled` `true` and a new reconcile interval
- **THEN** the system changes nothing and returns a conflict with `Turn on AI categorization before enabling it for a playlist`

#### Scenario: Enabling while categories are off
- **WHEN** AI and categorization are on, categories are off and a channel update supplies `categorization_enabled` `true`
- **THEN** the system changes nothing and returns a conflict with `Turn on AI categorization before enabling it for a channel`

#### Scenario: Disabling while AI is off
- **WHEN** AI is off and an update supplies `categorization_enabled` `false` for a channel whose categorization is on
- **THEN** the system stores the channel with `categorization_enabled` `false` and returns it

### Requirement: Categorization Setting On Creation
Creating a channel or a playlist SHALL require a `categorization_enabled` boolean. A missing or non-boolean value SHALL be rejected with a client error and nothing stored. Creating a source that already exists SHALL ignore the request's value and return the stored record.

#### Scenario: Created with categorization off
- **WHEN** a valid channel or playlist creation request supplies `categorization_enabled` `false`
- **THEN** the system stores the source with `categorization_enabled` `false` and returns it with `categorization_enabled` `false`

#### Scenario: Created without the setting
- **WHEN** a channel or playlist creation request omits `categorization_enabled`
- **THEN** the system stores nothing and rejects the request with a client error `Categorization setting must be true or false (missing)`

#### Scenario: Created with categorization on
- **WHEN** AI, categories and categorization are on and a valid channel or playlist creation request supplies `categorization_enabled` `true`
- **THEN** the system stores the source with `categorization_enabled` `true` and returns it with `categorization_enabled` `true`

#### Scenario: Duplicate source with a different setting
- **WHEN** AI, categories and categorization are on and a creation request names a channel or playlist already stored with `categorization_enabled` `false`, supplying `true`
- **THEN** the system makes no change and returns the existing record with `categorization_enabled` `false`

### Requirement: Categorization Setting In Responses
Every channel and playlist response (create, update, list) SHALL include the source's `categorization_enabled`.

#### Scenario: Listing sources
- **WHEN** a client lists channels or playlists and one of them has categorization enabled
- **THEN** that entry has `categorization_enabled` `true` and the others `false`
