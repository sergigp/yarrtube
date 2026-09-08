## Purpose

Keeps the stored videos for a tracked playlist synchronized with that playlist's actual contents on YouTube, first when the playlist is created and then automatically on a recurring interval for as long as the playlist exists.

## ADDED Requirements

### Requirement: Sync On Playlist Creation
The system SHALL sync a playlist's videos as soon as practical after the playlist is created.

#### Scenario: Newly created playlist gets synced
- **WHEN** a playlist is successfully created
- **THEN** the system fetches its current members from the YouTube Data API and persists them

### Requirement: Recurring Sync
The system SHALL schedule the next sync of a playlist after every sync attempt that runs to completion, at a configurable interval, regardless of whether any videos were added or removed during that attempt.

#### Scenario: Sync finds no changes
- **WHEN** a playlist sync completes and finds no new or removed videos
- **THEN** the system still schedules the next sync of that playlist after the configured interval

#### Scenario: Sync finds changes
- **WHEN** a playlist sync completes and finds new or removed videos
- **THEN** the system schedules the next sync of that playlist after the configured interval, in addition to persisting the changes

### Requirement: Configurable Sync Interval
The system SHALL read the recurring sync interval from a configuration value, defaulting to 3600 seconds when it is not set.

#### Scenario: Interval configured
- **WHEN** a sync interval configuration value is present
- **THEN** the system uses it as the delay before the next sync

#### Scenario: Interval not configured
- **WHEN** no sync interval configuration value is present
- **THEN** the system defaults to a 3600 second delay before the next sync

### Requirement: New Video Persistence
The system SHALL store each video found in a playlist that is not already stored for that playlist, with a status of PENDING.

#### Scenario: Playlist contains a video not yet stored
- **WHEN** a sync finds a video in the playlist that has no existing stored record for that playlist
- **THEN** the system stores it with status PENDING

#### Scenario: Playlist contains a video already stored
- **WHEN** a sync finds a video in the playlist that already has a stored record for that playlist
- **THEN** the system leaves that record's status unchanged

### Requirement: Removed Video Cleanup
The system SHALL delete a stored video's record when a sync finds that it is no longer present in the source playlist, and SHALL log each such deletion.

#### Scenario: Previously stored video no longer in playlist
- **WHEN** a sync finds that a video previously stored for a playlist is no longer a member of that playlist
- **THEN** the system deletes its stored record and logs the deletion

### Requirement: Sync Skipped For a Deleted Playlist
The system SHALL NOT fetch from YouTube, persist any videos, or schedule a further sync when a sync runs for a playlist that no longer exists.

#### Scenario: Scheduled sync runs after its playlist was deleted
- **WHEN** a sync is attempted for a playlist ID that no longer exists in storage
- **THEN** the system makes no YouTube request, persists nothing, and does not schedule another sync for that playlist ID
