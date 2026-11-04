## MODIFIED Requirements

### Requirement: Update Playlist Settings
The system SHALL provide an HTTP endpoint that changes whether a previously created playlist, identified by its ID, is excluded from home, whether its unwatched count is hidden, its reconcile interval and/or whether its videos are summarized, given an optional `exclude_from_home` boolean, an optional `hide_unwatched_count` boolean, an optional `reconcile_interval_minutes` and an optional `summaries_enabled` boolean, at least one of which SHALL be supplied. An absent field keeps its stored value. `summaries_enabled` `true` requires AI summaries on, as `source-summaries` describes. It SHALL change only those settings, leaving the playlist's other fields and its video records as they were, and SHALL return the updated playlist. Setting the values the playlist already has SHALL succeed without changing anything. A new reconcile interval SHALL apply from the next scheduled reconcile on: when the playlist's pending recurring reconcile is due later than the update time plus the new interval, the system SHALL move it to the update time plus the new interval; otherwise the pending reconcile keeps its time. The update publishes no domain event.

#### Scenario: Excluding a playlist from home
- **WHEN** a request sets `exclude_from_home` to `true` on a playlist that is shown on home
- **THEN** the system stores the playlist as excluded from home, returns it with `exclude_from_home` `true`, and leaves its name, path, quality, kind, reconcile interval, creation timestamp and video records unchanged

#### Scenario: Including a playlist in home again
- **WHEN** a request sets `exclude_from_home` to `false` on a playlist that is excluded from home
- **THEN** the system stores the playlist as shown on home, and returns it with `exclude_from_home` `false`

#### Scenario: Hiding a playlist's unwatched count
- **WHEN** a request sets `hide_unwatched_count` to `true` on a playlist whose unwatched count is shown
- **THEN** the system stores the playlist with its unwatched count hidden, returns it with `hide_unwatched_count` `true`, and leaves its other fields and video records unchanged

#### Scenario: Showing a playlist's unwatched count again
- **WHEN** a request sets `hide_unwatched_count` to `false` on a playlist whose unwatched count is hidden
- **THEN** the system stores the playlist with its unwatched count shown, and returns it with `hide_unwatched_count` `false`

#### Scenario: Changing the reconcile interval
- **WHEN** a request supplies `reconcile_interval_minutes` 60 for a playlist whose interval is 180
- **THEN** the system stores the playlist with a reconcile interval of 60 minutes, returns it, and leaves its other fields and video records unchanged

#### Scenario: Enabling summaries
- **WHEN** AI and summaries are on and a request supplies only `summaries_enabled` `true` for a playlist whose summaries are off
- **THEN** the system stores the playlist with `summaries_enabled` `true`, returns it, and leaves its other fields and video records unchanged

#### Scenario: Shortening the interval brings the pending reconcile forward
- **WHEN** a playlist's pending recurring reconcile is due in 20 hours and a request changes its interval to 60 minutes
- **THEN** the pending reconcile becomes due 60 minutes after the update

#### Scenario: Lengthening the interval keeps the pending reconcile
- **WHEN** a playlist's pending recurring reconcile is due in 30 minutes and a request changes its interval to 1440 minutes
- **THEN** the pending reconcile stays due in 30 minutes, and the reconcile after it is scheduled 1440 minutes after that one finishes

#### Scenario: Changing the interval while a reconcile is running
- **WHEN** a request changes a playlist's interval while a reconcile of that playlist is running and none is pending
- **THEN** the running reconcile, when it finishes, schedules the next one after the new interval

#### Scenario: Setting the values the playlist already has
- **WHEN** a request sets `exclude_from_home`, `hide_unwatched_count`, `reconcile_interval_minutes` and `summaries_enabled` to the values the playlist already has
- **THEN** the system succeeds and returns the playlist unchanged

#### Scenario: Unknown playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system makes no change and responds with not found and a meaningful error description

#### Scenario: Nothing to update
- **WHEN** a request supplies none of `exclude_from_home`, `hide_unwatched_count`, `reconcile_interval_minutes` and `summaries_enabled`
- **THEN** the system makes no change and returns a bad request with a meaningful error description

#### Scenario: Invalid exclude from home value
- **WHEN** a request supplies an `exclude_from_home` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Invalid hide unwatched count value
- **WHEN** a request supplies a `hide_unwatched_count` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Enabling summaries while AI summaries are off
- **WHEN** AI or summaries are off in the preferences and a request supplies `summaries_enabled` `true`, with or without other fields
- **THEN** the system makes no change and returns a conflict with `Turn on AI summaries before enabling them for a playlist`

#### Scenario: Invalid summaries value
- **WHEN** a request supplies a `summaries_enabled` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Invalid interval
- **WHEN** a request supplies a `reconcile_interval_minutes` that is not a whole number from 1 to 1440
- **THEN** the system makes no change and returns a bad request with the same error description as on creation
