## MODIFIED Requirements

### Requirement: List Channels
The system SHALL provide an HTTP endpoint that returns every currently stored channel, sorted alphabetically by name, ignoring case. Each returned channel SHALL include only its handle, name, path, quality, video limit and avatar filename (when present), plus its count of unwatched videos, counting only videos that have finished downloading.

#### Scenario: Channels exist
- **WHEN** one or more channels have been created
- **THEN** the system returns all of them, each with only its handle, name, path, quality, video limit, avatar filename (when present), and unwatched video count

#### Scenario: No channels exist
- **WHEN** no channels have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Unwatched count only includes downloaded, unwatched videos
- **WHEN** a channel has downloaded videos that are unwatched, downloaded videos that are watched, and videos that have not finished downloading
- **THEN** its unwatched video count equals the number of downloaded, unwatched videos only

#### Scenario: Channels are sorted by name regardless of creation order
- **WHEN** channels named "veritasium", "Kurzgesagt" and "3Blue1Brown" were created in that order
- **THEN** the system returns them in the order "3Blue1Brown", "Kurzgesagt", "veritasium"

## ADDED Requirements

### Requirement: Update Channel Settings
The system SHALL provide an HTTP endpoint that changes the quality and/or video limit of a stored channel identified by its handle, and returns the channel as stored. Each field is optional and validated as in Create Channel; an absent field keeps its stored value, and at least one SHALL be supplied. The handle, name, path and avatar SHALL NOT change. The change applies from the channel's next sync and publishes no domain event.

#### Scenario: Changing the quality
- **WHEN** a request supplies quality `low` for a stored channel whose quality is `high`
- **THEN** the system stores the channel with quality `low` and its other fields unchanged, and returns it

#### Scenario: Changing the video limit
- **WHEN** a request supplies video limit 20 for a stored channel whose video limit is 5
- **THEN** the system stores the channel with video limit 20 and its other fields unchanged, and returns it

#### Scenario: Changing both settings
- **WHEN** a request supplies both a quality and a video limit for a stored channel
- **THEN** the system stores both new values and returns the updated channel

#### Scenario: Setting the values the channel already has
- **WHEN** a request supplies the quality and video limit the channel already has
- **THEN** the system makes no change and returns the channel as stored

#### Scenario: Nothing to update
- **WHEN** a request supplies neither a quality nor a video limit
- **THEN** the system rejects the request without changing storage and returns a bad request with a meaningful error description

#### Scenario: Invalid quality or video limit
- **WHEN** a request supplies a quality that is not `high`, `mid` or `low`, or a video limit that is not an integer from 1 to 1000 inclusive
- **THEN** the system rejects the request without changing storage and returns a bad request with the same error description as Create Channel

#### Scenario: Updating a nonexistent channel
- **WHEN** a request identifies a channel handle that does not exist in storage
- **THEN** the system makes no change to storage and returns a not found response with a meaningful error description

#### Scenario: New quality applies to later downloads only
- **WHEN** a channel's quality is changed and the channel then syncs and finds a new video
- **THEN** that video is downloaded at the new quality, while videos already downloaded or already queued for download keep the quality they had

#### Scenario: New video limit applies at the next sync
- **WHEN** a channel's video limit is changed and the channel then syncs
- **THEN** the sync keeps the channel's most recent uploads up to the new limit, downloading newly included videos and removing videos beyond it as `channel-video-sync` describes
