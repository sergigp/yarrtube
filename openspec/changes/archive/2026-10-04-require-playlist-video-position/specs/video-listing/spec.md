## MODIFIED Requirements

### Requirement: List Videos For A Playlist
The system SHALL provide an HTTP endpoint that, given a tracked playlist's ID, returns every video recorded for that playlist except those that are excluded, including each video's ID, title, status, quality (when downloaded), filename (when downloaded), thumbnail filename (when present), duration in seconds (when present), whether it has been watched, and its saved playback position in seconds. The returned videos SHALL be ordered by their current position in the source YouTube playlist.

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

#### Scenario: Excluded videos are omitted
- **WHEN** a client requests the videos of a tracked playlist that has an excluded video alongside other videos
- **THEN** the returned list contains the other videos and does not contain the excluded one
