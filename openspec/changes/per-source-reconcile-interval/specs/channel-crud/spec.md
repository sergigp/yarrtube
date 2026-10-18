## ADDED Requirements

### Requirement: Channel Reconcile Interval
Every stored channel SHALL have a reconcile interval: a whole number of minutes from 1 to 1440 (24 hours), setting how long after one recurring reconcile of the channel finishes the next one runs. Creating a channel SHALL accept an optional `reconcile_interval_minutes`; when it is omitted the channel is stored with 180 minutes (3 hours). Channels stored before reconcile intervals existed SHALL have 180 minutes. The created channel's response SHALL include its `reconcile_interval_minutes`.

#### Scenario: Creating a channel without an interval
- **WHEN** a valid creation request omits `reconcile_interval_minutes`
- **THEN** the system persists the channel with a reconcile interval of 180 minutes, and returns it with `reconcile_interval_minutes` 180

#### Scenario: Creating a channel with an interval
- **WHEN** a valid creation request supplies `reconcile_interval_minutes` 30
- **THEN** the system persists the channel with a reconcile interval of 30 minutes, and returns it with `reconcile_interval_minutes` 30

#### Scenario: Invalid interval on creation
- **WHEN** a creation request supplies a `reconcile_interval_minutes` that is not a whole number from 1 to 1440
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request whose error description states the bound it breaks (at least 1 minute, at most 1440 minutes) and the value received

#### Scenario: Duplicate channel handle with a different interval
- **WHEN** a request supplies a `channel` value whose handle already exists in storage, with a `reconcile_interval_minutes` different from the stored record's
- **THEN** the system makes no change, ignores the request's interval, and returns the existing record with its original interval

#### Scenario: Channels existing before the upgrade
- **WHEN** the system starts on a database whose channels were stored before reconcile intervals existed
- **THEN** every such channel has a reconcile interval of 180 minutes

## MODIFIED Requirements

### Requirement: List Channels
The system SHALL provide an HTTP endpoint that returns every currently stored channel, sorted alphabetically by name, ignoring case. Each returned channel SHALL include only its handle, name, path, quality, video limit, reconcile interval in minutes and avatar filename (when present), plus its count of unwatched videos, counting only videos that have finished downloading.

#### Scenario: Channels exist
- **WHEN** one or more channels have been created
- **THEN** the system returns all of them, each with only its handle, name, path, quality, video limit, reconcile interval in minutes, avatar filename (when present), and unwatched video count

#### Scenario: No channels exist
- **WHEN** no channels have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Unwatched count only includes downloaded, unwatched videos
- **WHEN** a channel has downloaded videos that are unwatched, downloaded videos that are watched, and videos that have not finished downloading
- **THEN** its unwatched video count equals the number of downloaded, unwatched videos only

#### Scenario: Channels are sorted by name regardless of creation order
- **WHEN** channels named "veritasium", "Kurzgesagt" and "3Blue1Brown" were created in that order
- **THEN** the system returns them in the order "3Blue1Brown", "Kurzgesagt", "veritasium"

### Requirement: Update Channel Settings
The system SHALL provide an HTTP endpoint that changes the quality, video limit and/or reconcile interval of a stored channel identified by its handle, and returns the channel as stored. Each field is optional and validated as on creation; an absent field keeps its stored value, and at least one SHALL be supplied. The handle, name, path and avatar SHALL NOT change. Quality and video limit apply from the channel's next sync. A new reconcile interval SHALL apply from the next scheduled reconcile on: when the channel's pending recurring reconcile is due later than the update time plus the new interval, the system SHALL move it to the update time plus the new interval; otherwise the pending reconcile keeps its time. The update publishes no domain event.

#### Scenario: Changing the quality
- **WHEN** a request supplies quality `low` for a stored channel whose quality is `high`
- **THEN** the system stores the channel with quality `low` and its other fields unchanged, and returns it

#### Scenario: Changing the video limit
- **WHEN** a request supplies video limit 20 for a stored channel whose video limit is 5
- **THEN** the system stores the channel with video limit 20 and its other fields unchanged, and returns it

#### Scenario: Changing the reconcile interval
- **WHEN** a request supplies `reconcile_interval_minutes` 60 for a stored channel whose interval is 180
- **THEN** the system stores the channel with a reconcile interval of 60 minutes and its other fields unchanged, and returns it

#### Scenario: Shortening the interval brings the pending reconcile forward
- **WHEN** a channel's pending recurring reconcile is due in 20 hours and a request changes its interval to 60 minutes
- **THEN** the pending reconcile becomes due 60 minutes after the update

#### Scenario: Lengthening the interval keeps the pending reconcile
- **WHEN** a channel's pending recurring reconcile is due in 30 minutes and a request changes its interval to 1440 minutes
- **THEN** the pending reconcile stays due in 30 minutes, and the reconcile after it is scheduled 1440 minutes after that one finishes

#### Scenario: Changing the interval while a reconcile is running
- **WHEN** a request changes a channel's interval while a reconcile of that channel is running and none is pending
- **THEN** the running reconcile, when it finishes, schedules the next one after the new interval

#### Scenario: Changing both settings
- **WHEN** a request supplies a quality, a video limit and a reconcile interval for a stored channel
- **THEN** the system stores all new values and returns the updated channel

#### Scenario: Setting the values the channel already has
- **WHEN** a request supplies the quality, video limit and reconcile interval the channel already has
- **THEN** the system makes no change and returns the channel as stored

#### Scenario: Nothing to update
- **WHEN** a request supplies none of quality, video limit and reconcile interval
- **THEN** the system rejects the request without changing storage and returns a bad request with a meaningful error description

#### Scenario: Invalid quality or video limit
- **WHEN** a request supplies a quality that is not `high`, `mid` or `low`, a video limit that is not an integer from 1 to 1000 inclusive, or a reconcile interval that is not a whole number from 1 to 1440
- **THEN** the system rejects the request without changing storage and returns a bad request with the same error description as on creation

#### Scenario: Updating a nonexistent channel
- **WHEN** a request identifies a channel handle that does not exist in storage
- **THEN** the system makes no change to storage and returns a not found response with a meaningful error description

#### Scenario: New quality applies to later downloads only
- **WHEN** a channel's quality is changed and the channel then syncs and finds a new video
- **THEN** that video is downloaded at the new quality, while videos already downloaded or already queued for download keep the quality they had

#### Scenario: New video limit applies at the next sync
- **WHEN** a channel's video limit is changed and the channel then syncs
- **THEN** the sync keeps the channel's most recent uploads up to the new limit, downloading newly included videos and removing videos beyond it as `channel-video-sync` describes
