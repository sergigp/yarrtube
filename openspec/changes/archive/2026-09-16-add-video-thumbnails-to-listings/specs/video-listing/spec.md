## MODIFIED Requirements

### Requirement: List Recently Synced Videos Across Sources
The system SHALL provide an HTTP endpoint that returns downloaded videos across every tracked playlist and channel combined into a single list, ordered by sync time (the time the video was recorded by the system), newest first. The endpoint SHALL accept an optional limit on the number of videos returned, defaulting to 20 and capped at 100 when a larger value is requested. Each returned video SHALL include its ID, title, thumbnail filename (when present), and the source that tracks it (kind: `playlist` or `channel`, that source's identifier, and that source's storage path). A video tracked by more than one source SHALL appear once per source.

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
