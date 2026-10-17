## MODIFIED Requirements

### Requirement: Recurring Reconciliation
The system SHALL schedule the next reconcile of a playlist after every
reconcile attempt that runs to completion, after the playlist's own reconcile
interval as stored when the attempt finishes, regardless of the playlist's
kind, whether any videos were added or removed, or whether any file was
healed. When the playlist no longer exists when the attempt finishes, the
system SHALL schedule no further reconcile of it.

#### Scenario: Reconcile finds no changes
- **WHEN** a playlist reconcile completes and finds no membership or filesystem changes
- **THEN** the system still schedules the next reconcile of that playlist after the playlist's reconcile interval

#### Scenario: Reconcile finds changes
- **WHEN** a playlist reconcile completes and finds membership and/or filesystem changes
- **THEN** the system schedules the next reconcile of that playlist after the playlist's reconcile interval, in addition to persisting the changes

#### Scenario: Playlists reconcile on their own intervals
- **WHEN** a playlist with a 30 minute interval and a playlist with a 1440 minute interval each complete a reconcile at the same time
- **THEN** the next reconcile of the first is scheduled 30 minutes later and of the second 1440 minutes later

#### Scenario: Interval changed during a reconcile
- **WHEN** a playlist's interval is changed from 180 to 60 minutes while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules the next one 60 minutes later

#### Scenario: Playlist deleted during a reconcile
- **WHEN** a playlist is deleted while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules no further reconcile of the playlist

## REMOVED Requirements

### Requirement: Configurable Reconcile Interval
**Reason**: The interval is now a per-playlist and per-channel setting (see `playlist-crud` Playlist Reconcile Interval and `channel-crud` Channel Reconcile Interval).
**Migration**: Remove `YARRTUBE_RECONCILE_INTERVAL_SECONDS` from the environment; it is ignored. Existing sources get 180 minutes; set a different interval per source from the web UI or the update endpoints.
