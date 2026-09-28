## ADDED Requirements

### Requirement: List Continue Watching Videos Across Sources
The system SHALL provide an HTTP endpoint that returns, across every tracked playlist and channel, the downloaded videos a user has started and not finished recently: unwatched, with a saved playback position greater than 30 seconds, and last played within the past 7 days. Videos SHALL be ordered by last played time, most recent first. A YouTube video tracked by more than one source SHALL appear only once. The endpoint SHALL accept an optional limit on the number of videos returned, defaulting to 20 and capped at 100 when a larger value is requested. Each returned video SHALL include the same fields as a recently synced video, plus its saved playback position in seconds.

#### Scenario: A recently started video is listed
- **WHEN** a downloaded, unwatched video has a saved position of 120 seconds and was last played 2 days ago
- **THEN** the daemon responds with HTTP status 200 and a list that includes that video with its saved playback position

#### Scenario: Most recently played first
- **WHEN** two videos qualify, one last played 1 hour ago and one last played 3 days ago
- **THEN** the one played 1 hour ago comes first

#### Scenario: A video not played for over a week is excluded
- **WHEN** an unwatched video with a saved position was last played more than 7 days ago
- **THEN** it is excluded from the list

#### Scenario: A video barely started is excluded
- **WHEN** an unwatched video's saved position is 30 seconds or less
- **THEN** it is excluded from the list

#### Scenario: A watched video is excluded
- **WHEN** a video is watched
- **THEN** it is excluded from the list

#### Scenario: A video never played is excluded
- **WHEN** a video has no last played time
- **THEN** it is excluded from the list

#### Scenario: A non-downloaded video is excluded
- **WHEN** a video that otherwise qualifies has not finished downloading
- **THEN** it is excluded from the list

#### Scenario: A video tracked by two sources appears once
- **WHEN** a qualifying YouTube video is tracked by both a channel and a playlist
- **THEN** the list includes it once

#### Scenario: Nothing to continue
- **WHEN** no video qualifies
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Limit narrows the result
- **WHEN** a client requests continue watching videos with a limit of N
- **THEN** the daemon returns at most N videos

#### Scenario: Requested limit is capped
- **WHEN** a client requests continue watching videos with a limit greater than 100
- **THEN** the daemon returns at most 100 videos

### Requirement: List Quick Watch Videos Across Sources
The system SHALL provide an HTTP endpoint that returns, across every tracked playlist and channel, the downloaded, unwatched videos with a recorded duration shorter than 15 minutes (900 seconds), ordered by sync time, newest first. A YouTube video tracked by more than one source SHALL appear only once. The endpoint SHALL accept an optional limit on the number of videos returned, defaulting to 20 and capped at 100 when a larger value is requested. Each returned video SHALL include the same fields as a recently synced video, plus its saved playback position in seconds.

#### Scenario: A short unwatched video is listed
- **WHEN** a downloaded, unwatched video lasts 10 minutes
- **THEN** the daemon responds with HTTP status 200 and a list that includes that video

#### Scenario: Newest sync first
- **WHEN** two short unwatched videos were synced at different times
- **THEN** the more recently synced one comes first

#### Scenario: A video of 15 minutes or longer is excluded
- **WHEN** an unwatched video lasts 900 seconds or more
- **THEN** it is excluded from the list

#### Scenario: A video without a recorded duration is excluded
- **WHEN** an unwatched video has no recorded duration
- **THEN** it is excluded from the list

#### Scenario: A watched short video is excluded
- **WHEN** a short video is watched
- **THEN** it is excluded from the list

#### Scenario: A non-downloaded short video is excluded
- **WHEN** a short video has not finished downloading
- **THEN** it is excluded from the list

#### Scenario: A short video tracked by two sources appears once
- **WHEN** a short, unwatched YouTube video is tracked by both a channel and a playlist
- **THEN** the list includes it once

#### Scenario: No quick watches
- **WHEN** no video qualifies
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: Limit narrows the result
- **WHEN** a client requests quick watch videos with a limit of N
- **THEN** the daemon returns at most N videos

#### Scenario: Requested limit is capped
- **WHEN** a client requests quick watch videos with a limit greater than 100
- **THEN** the daemon returns at most 100 videos

### Requirement: List Home Videos Across Sources
The system SHALL provide an HTTP endpoint that returns, in one response, the videos of the three home sections, so that a YouTube video appears in at most one section. "Continue watching" SHALL hold up to 6 videos chosen and ordered as in the continue watching listing. "Quick watches" SHALL hold up to 6 videos chosen and ordered as in the quick watches listing, excluding the videos shown under "Continue watching". "Latest videos" SHALL hold up to 18 downloaded videos ordered by sync time, newest first, once per source, excluding the videos shown under "Continue watching" or "Quick watches". Only the videos a section shows SHALL be excluded from the sections after it. Each returned video SHALL include the same fields as a continue watching video.

#### Scenario: Each video in its own section
- **WHEN** one downloaded video is in progress, another is a quick watch and a third is neither
- **THEN** the daemon responds with HTTP status 200, and each video appears only in its own section

#### Scenario: A started short video counts as continue watching
- **WHEN** a video qualifies for both "Continue watching" and "Quick watches"
- **THEN** it appears under "Continue watching" only

#### Scenario: A video shown above is not repeated in latest videos
- **WHEN** a video appears under "Continue watching" or "Quick watches"
- **THEN** it does not appear under "Latest videos"

#### Scenario: Sections are bounded
- **WHEN** more videos qualify for a section than it holds
- **THEN** "Continue watching" and "Quick watches" hold at most 6 videos each, and "Latest videos" at most 18

#### Scenario: A video left out of a full section can appear further down
- **WHEN** a video qualifies for a section that is already full
- **THEN** it can appear in a later section it qualifies for

