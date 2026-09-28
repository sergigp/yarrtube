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
The system SHALL provide an HTTP endpoint that returns downloaded videos across every tracked playlist and channel combined into a single list, ordered by sync time (the time the video was recorded by the system), newest first. The endpoint SHALL accept an optional limit on the number of videos returned, defaulting to 20 and capped at 100 when a larger value is requested. Each returned video SHALL include its ID, title, thumbnail filename (when present), duration in seconds (when present), whether it has been watched, and the source that tracks it (kind: `playlist` or `channel`, that source's identifier, that source's display name, that source's storage path, and — for a channel source — that channel's avatar filename, when present). A video tracked by more than one source SHALL appear once per source.

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

#### Scenario: Channel source includes the channel's name
- **WHEN** a returned video's source is a channel
- **THEN** the source object includes that channel's name

#### Scenario: Playlist source includes the playlist's name
- **WHEN** a returned video's source is a playlist
- **THEN** the source object includes that playlist's name

### Requirement: Listed Videos Include Sync Time
The system SHALL include each video's recorded sync time in the responses that list the videos of a playlist and of a channel, and SHALL report it as absent for a video that has none.

#### Scenario: Downloaded video is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a downloaded video
- **THEN** that video's entry includes its sync time

#### Scenario: Video without a sync time is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video with no recorded sync time
- **THEN** that video's entry reports its sync time as absent

### Requirement: Listed Videos Include Their Metadata
The system SHALL include each video's publish time, description and channel name, taken from its generated metadata, in the responses that list the videos of a playlist and of a channel. The description SHALL be the same truncated plot recorded for the video's metadata file. For a video whose metadata has not been generated, the system SHALL report all three as absent.

#### Scenario: Video with generated metadata is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video with generated metadata
- **THEN** that video's entry includes its publish time, its description and the name of the channel that published it

#### Scenario: Video without generated metadata is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video whose metadata has not been generated
- **THEN** that video's entry reports its publish time, description and channel name as absent, and every other field is still returned

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
