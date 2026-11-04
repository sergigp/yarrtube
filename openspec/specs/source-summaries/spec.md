# source-summaries Specification

## Purpose

Lets each tracked channel and playlist opt in to AI video summaries, so summary generation only runs for the sources the user chose.

## Requirements

### Requirement: Source Summaries Setting
Every stored channel and playlist SHALL have a `summaries_enabled` boolean, `false` unless set. Sources stored before this setting existed SHALL have `false`. It has no other effect yet.

#### Scenario: Existing sources after upgrade
- **WHEN** the daemon starts on a database with channels and playlists stored before this setting existed
- **THEN** every one of them has `summaries_enabled` `false`

### Requirement: Summaries Need AI Summaries On
A create or update request for a channel or playlist that supplies `summaries_enabled` `true` SHALL be rejected with a conflict response, changing nothing, unless the preferences have both AI and summaries on. The error SHALL read `Turn on AI summaries before enabling them for a channel` (or `for a playlist`). On creation the check SHALL run before YouTube is looked up. `summaries_enabled` `false` SHALL always be accepted.

#### Scenario: Creating with summaries while AI is off
- **WHEN** AI is off and a channel creation request supplies `summaries_enabled` `true`
- **THEN** the system stores nothing, does not look the channel up on YouTube, and returns a conflict with `Turn on AI summaries before enabling them for a channel`

#### Scenario: Enabling while summaries are off
- **WHEN** AI is on, summaries are off and a playlist update supplies `summaries_enabled` `true` and a new reconcile interval
- **THEN** the system changes nothing and returns a conflict with `Turn on AI summaries before enabling them for a playlist`

#### Scenario: Disabling while AI is off
- **WHEN** AI is off and an update supplies `summaries_enabled` `false` for a channel whose summaries are on
- **THEN** the system stores the channel with `summaries_enabled` `false` and returns it

### Requirement: Summaries Setting On Creation
Creating a channel or a playlist SHALL require a `summaries_enabled` boolean. A missing or non-boolean value SHALL be rejected with a client error and nothing stored. Creating a source that already exists SHALL ignore the request's value and return the stored record.

#### Scenario: Created with summaries off
- **WHEN** a valid channel or playlist creation request supplies `summaries_enabled` `false`
- **THEN** the system stores the source with `summaries_enabled` `false` and returns it with `summaries_enabled` `false`

#### Scenario: Created without the setting
- **WHEN** a channel or playlist creation request omits `summaries_enabled`
- **THEN** the system stores nothing and rejects the request with a client error `Summaries setting must be true or false (missing)`

#### Scenario: Created with summaries on
- **WHEN** AI and summaries are on and a valid channel or playlist creation request supplies `summaries_enabled` `true`
- **THEN** the system stores the source with `summaries_enabled` `true` and returns it with `summaries_enabled` `true`

#### Scenario: Duplicate source with a different setting
- **WHEN** AI and summaries are on and a creation request names a channel or playlist already stored with `summaries_enabled` `false`, supplying `true`
- **THEN** the system makes no change and returns the existing record with `summaries_enabled` `false`

### Requirement: Summaries Setting In Responses
Every channel and playlist response (create, update, list) SHALL include the source's `summaries_enabled`.

#### Scenario: Listing sources
- **WHEN** a client lists channels or playlists and one of them has summaries enabled
- **THEN** that entry has `summaries_enabled` `true` and the others `false`
