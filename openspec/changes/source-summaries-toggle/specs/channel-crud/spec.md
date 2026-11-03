## MODIFIED Requirements

### Requirement: Update Channel Settings
The system SHALL provide an HTTP endpoint that changes the quality, video limit, reconcile interval and/or summaries setting (`summaries_enabled`, a boolean) of a stored channel identified by its handle, and returns the channel as stored. Each field is optional and validated as on creation; `summaries_enabled` `true` requires AI summaries on, as `source-summaries` describes; an absent field keeps its stored value, and at least one SHALL be supplied. The handle, name, path and avatar SHALL NOT change. Quality and video limit apply from the channel's next sync. A new reconcile interval SHALL apply from the next scheduled reconcile on: when the channel's pending recurring reconcile is due later than the update time plus the new interval, the system SHALL move it to the update time plus the new interval; otherwise the pending reconcile keeps its time. The update publishes no domain event.

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

#### Scenario: Enabling summaries
- **WHEN** AI and summaries are on and a request supplies only `summaries_enabled` `true` for a stored channel whose summaries are off
- **THEN** the system stores the channel with `summaries_enabled` `true` and its other fields unchanged, and returns it

#### Scenario: Enabling summaries while AI summaries are off
- **WHEN** AI or summaries are off in the preferences and a request supplies `summaries_enabled` `true`, with or without other fields
- **THEN** the system makes no change and returns a conflict with `Turn on AI summaries before enabling them for a channel`

#### Scenario: Invalid summaries value
- **WHEN** a request supplies a `summaries_enabled` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error

#### Scenario: Changing both settings
- **WHEN** a request supplies a quality, a video limit and a reconcile interval for a stored channel
- **THEN** the system stores all new values and returns the updated channel

#### Scenario: Setting the values the channel already has
- **WHEN** a request supplies the quality, video limit, reconcile interval and summaries setting the channel already has
- **THEN** the system makes no change and returns the channel as stored

#### Scenario: Nothing to update
- **WHEN** a request supplies none of quality, video limit, reconcile interval and `summaries_enabled`
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
