# video-listing Specification

## Purpose

Provides an HTTP endpoint to list the videos belonging to a single tracked playlist, so a playlist's contents can be browsed.

## Requirements

### Requirement: List Videos For A Playlist
The system SHALL provide an HTTP endpoint that, given a tracked playlist's ID, returns every video recorded for that playlist, including each video's ID, title, status, quality (when downloaded), filename (when downloaded), thumbnail filename (when present), duration in seconds (when present), whether it has been watched, and its saved playback position in seconds. For a YouTube-linked playlist, the returned videos SHALL be ordered by their current position in the source YouTube playlist. For a custom playlist, no particular order is guaranteed.

#### Scenario: Playlist has videos
- **WHEN** a client requests the videos of a tracked playlist that has one or more recorded videos
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those videos

#### Scenario: Playlist has no videos
- **WHEN** a client requests the videos of a tracked playlist that has no recorded videos
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Playlist does not exist
- **WHEN** a client requests the videos of a playlist ID that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and does not return a list

#### Scenario: YouTube-linked playlist's videos are ordered by playlist position
- **WHEN** a client requests the videos of a YouTube-linked playlist
- **THEN** the daemon returns them ordered by their position in the source YouTube playlist, matching the order they appear in on YouTube

#### Scenario: Playlist videos include their watch state
- **WHEN** a client requests the videos of a tracked playlist that has a watched video and a partly watched video
- **THEN** each returned video reports whether it has been watched and its saved playback position

### Requirement: List Videos For A Channel
The system SHALL provide an HTTP endpoint that, given a tracked channel's handle, returns every video recorded for that channel, including each video's ID, title, status, quality (when downloaded), filename (when downloaded), thumbnail filename (when present), duration in seconds (when present), whether it has been watched, and its saved playback position in seconds, ordered by recency (most recently uploaded first).

#### Scenario: Channel has videos
- **WHEN** a client requests the videos of a tracked channel that has one or more recorded videos
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those videos, ordered most recent first

#### Scenario: Channel has no videos
- **WHEN** a client requests the videos of a tracked channel that has no recorded videos
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Channel does not exist
- **WHEN** a client requests the videos of a channel handle that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and does not return a list

#### Scenario: Channel videos include their watch state
- **WHEN** a client requests the videos of a tracked channel that has a watched video and a partly watched video
- **THEN** each returned video reports whether it has been watched and its saved playback position

### Requirement: List Recently Synced Videos Across Sources
The system SHALL provide an HTTP endpoint that returns downloaded videos across every tracked playlist and channel combined into a single list, ordered by sync time (the time the video was recorded by the system), newest first. The endpoint SHALL accept an optional limit on the number of videos returned, defaulting to 20 and capped at 100 when a larger value is requested. Each returned video SHALL include its ID, title, thumbnail filename (when present), duration in seconds (when present), whether it has been watched, and the source that tracks it (kind: `playlist` or `channel`, that source's identifier, that source's storage path, and — for a channel source — that channel's avatar filename, when present). A video tracked by more than one source SHALL appear once per source.

#### Scenario: Recently synced videos exist
- **WHEN** a client requests recently synced videos and at least one downloaded video is recorded
- **THEN** the daemon responds with HTTP status 200 and a list of videos ordered by sync time, newest first

#### Scenario: No recently synced videos
- **WHEN** a client requests recently synced videos and no downloaded video is recorded
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Limit narrows the result
- **WHEN** a client requests recently synced videos with a limit of N
- **THEN** the daemon returns at most N videos

#### Scenario: Default limit applied when omitted
- **WHEN** a client requests recently synced videos without specifying a limit
- **THEN** the daemon returns at most 20 videos

#### Scenario: Requested limit is capped
- **WHEN** a client requests recently synced videos with a limit greater than 100
- **THEN** the daemon returns at most 100 videos

#### Scenario: Non-downloaded videos are excluded
- **WHEN** a synced video has not finished downloading
- **THEN** it is excluded from the recently synced videos list

#### Scenario: Videos from both playlists and channels are combined
- **WHEN** downloaded videos are recorded by both a tracked playlist and a tracked channel
- **THEN** the returned list includes videos from both sources, each tagged with its own source kind, identifier, and storage path

#### Scenario: A video tracked by more than one source appears once per source
- **WHEN** the same YouTube video is tracked by two different sources
- **THEN** the returned list includes one entry per source that tracks it

#### Scenario: Recorded thumbnail is included
- **WHEN** a returned video has a recorded thumbnail file
- **THEN** its thumbnail filename is included in the response

#### Scenario: Missing thumbnail is omitted
- **WHEN** a returned video has no recorded thumbnail file
- **THEN** its thumbnail filename is absent from the response

#### Scenario: Recorded duration is included
- **WHEN** a returned video has a recorded duration
- **THEN** its duration, in seconds, is included in the response

#### Scenario: Missing duration is omitted
- **WHEN** a returned video has no recorded duration
- **THEN** its duration is absent from the response

#### Scenario: Channel source includes the channel's avatar filename
- **WHEN** a returned video's source is a channel that has a recorded avatar filename
- **THEN** the source object includes that avatar filename

#### Scenario: Playlist source has no avatar filename
- **WHEN** a returned video's source is a playlist
- **THEN** the source object's avatar filename is absent

#### Scenario: Recent videos include whether they have been watched
- **WHEN** a returned recently synced video has been watched
- **THEN** it is reported as watched
