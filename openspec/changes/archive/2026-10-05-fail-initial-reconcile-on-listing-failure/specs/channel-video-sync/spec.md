## MODIFIED Requirements

### Requirement: Channel Video Discovery Via yt-dlp
The system SHALL discover a channel's current videos by invoking `yt-dlp`
against that channel's videos listing, requesting only its `video_limit`
most recent uploads, and parsing each discovered video's ID and title from
structured (JSON) output rather than delimited text, so that a video title
containing arbitrary characters (including a `|` or other delimiter-like
character) cannot corrupt discovery of that video or any other.

#### Scenario: Channel has more uploads than its video limit
- **WHEN** a reconcile pass discovers a channel's videos
- **THEN** the system requests and receives at most `video_limit` videos, ordered most recent first

#### Scenario: A discovered video's title contains a delimiter-like character
- **WHEN** a reconcile pass discovers a video whose title contains a `|` character (or other text that would corrupt a delimited-text parse)
- **THEN** the system correctly records that video's exact title and ID, and no other discovered video's data is corrupted by it

#### Scenario: yt-dlp fails to list a channel's videos
- **WHEN** a recurring or on-demand reconcile pass's attempt to discover a channel's current videos fails (e.g. the channel is private, terminated, or `yt-dlp` errors)
- **THEN** the system does not alter any of that channel's stored videos, logs the failure, and does not treat it as a fatal error for the reconcile pass

### Requirement: Reconcile On Channel Creation
The system SHALL run one reconcile pass for a channel as soon as practical after it is created. If discovering the channel's current videos fails, that pass SHALL fail, so it is retried with the creation event's processing rather than leaving the new channel empty until a recurring pass.

#### Scenario: Newly created channel gets reconciled
- **WHEN** a channel is successfully created
- **THEN** the system discovers its current most-recent videos (up to `video_limit`) and persists them

#### Scenario: Discovery fails during the initial reconcile
- **WHEN** the initial reconcile pass of a newly created channel fails to discover the channel's current videos
- **THEN** the system stores no video, publishes no event, schedules no recurring reconcile of the channel, and fails the `ChannelCreated` event's processing so it is retried
