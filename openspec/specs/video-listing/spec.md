# video-listing Specification

## Purpose

Provides an HTTP endpoint to list the videos belonging to a single tracked playlist, so a playlist's contents can be browsed.

## Requirements

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

### Requirement: List Videos For A Channel
The system SHALL provide an HTTP endpoint that, given a tracked channel's handle, returns every video recorded for that channel except those that are excluded, including each video's ID, title, status, quality (when downloaded), filename (when downloaded), thumbnail filename (when present), duration in seconds (when present), whether it has been watched, and its saved playback position in seconds, ordered by recency (most recently uploaded first).

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

#### Scenario: Excluded videos are omitted
- **WHEN** a client requests the videos of a tracked channel that has an excluded video alongside other videos
- **THEN** the returned list contains the other videos and does not contain the excluded one

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

### Requirement: List Home Videos Across Sources
The system SHALL provide an HTTP endpoint that returns, in one response, the downloaded videos of the three home sections across every tracked channel and every tracked playlist not excluded from home, so that a YouTube video appears in at most one section.

A playlist excluded from home SHALL NOT be the source of any video in any section. A video it tracks SHALL still appear when another source that is not excluded tracks it, through that source.

"Continue watching" SHALL hold up to 6 videos the user has started and not finished recently:
- each is unwatched, with a saved playback position greater than 30 seconds, and was last played within the past 7 days
- ordered by last played time, most recent first

"Quick watches" SHALL hold up to 6 short videos:
- each is unwatched, with a recorded duration shorter than 15 minutes (900 seconds), and not shown under "Continue watching"
- ordered by sync time (the time the video was recorded by the system), newest first

In these two sections, a YouTube video tracked by more than one source SHALL appear once, from a tracking channel when there is one.

"Latest videos" SHALL hold up to 18 videos that are not shown under "Continue watching" or "Quick watches", ordered by sync time, newest first, once per source that tracks them.

Only the videos a section shows SHALL be left out of the sections after it.

Each returned video SHALL include:
- its ID and title
- its thumbnail filename and duration in seconds, when present
- whether it has been watched, and its saved playback position in seconds
- the source that tracks it: kind (`playlist` or `channel`), identifier, display name, storage path, and, for a channel source, that channel's avatar filename when present

#### Scenario: Each video in its own section
- **WHEN** one downloaded video is in progress, another is a quick watch and a third is neither
- **THEN** the daemon responds with HTTP status 200, and each video appears only in its own section

#### Scenario: Nothing downloaded
- **WHEN** no downloaded video is recorded
- **THEN** the daemon responds with HTTP status 200 and three empty sections

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

#### Scenario: Continue watching is ordered by last played time
- **WHEN** two videos qualify for "Continue watching", one last played 1 hour ago and one 3 days ago
- **THEN** the one played 1 hour ago comes first

#### Scenario: A video not played for over a week is not continued
- **WHEN** an unwatched, downloaded video with a saved position was last played more than 7 days ago
- **THEN** it does not appear under "Continue watching", and appears under "Latest videos" instead

#### Scenario: A barely started video is not continued
- **WHEN** an unwatched video's saved position is 30 seconds or less
- **THEN** it does not appear under "Continue watching"

#### Scenario: A watched or never played video is not continued
- **WHEN** a video is watched, or has no last played time
- **THEN** it does not appear under "Continue watching"

#### Scenario: Quick watches are ordered by sync time
- **WHEN** two short, unwatched videos were synced at different times
- **THEN** the more recently synced one comes first under "Quick watches"

#### Scenario: A video of 15 minutes or longer is not a quick watch
- **WHEN** an unwatched video lasts 900 seconds or more
- **THEN** it does not appear under "Quick watches"

#### Scenario: A video without a recorded duration is not a quick watch
- **WHEN** an unwatched video has no recorded duration
- **THEN** it does not appear under "Quick watches"

#### Scenario: A watched short video is not a quick watch
- **WHEN** a short video is watched
- **THEN** it does not appear under "Quick watches"

#### Scenario: Non-downloaded videos are excluded
- **WHEN** a video has not finished downloading
- **THEN** it appears in no section

#### Scenario: A video tracked by two sources appears once above latest videos
- **WHEN** a video that qualifies for "Continue watching" or "Quick watches" is tracked by both a channel and a playlist
- **THEN** it appears once, from the channel

#### Scenario: Latest videos lists a video once per source
- **WHEN** a video under "Latest videos" is tracked by two different sources
- **THEN** it appears once per source that tracks it

#### Scenario: Latest videos are ordered by sync time
- **WHEN** videos under "Latest videos" were synced at different times
- **THEN** they are ordered newest first

#### Scenario: Card fields
- **WHEN** a video is returned in any section
- **THEN** it includes its ID, title, thumbnail filename and duration when present, whether it has been watched, its saved playback position, and its source's kind, identifier, name and storage path

#### Scenario: Channel source includes the channel's avatar filename
- **WHEN** a returned video's source is a channel that has a recorded avatar filename
- **THEN** the source includes that avatar filename

#### Scenario: Playlist source has no avatar filename
- **WHEN** a returned video's source is a playlist
- **THEN** the source's avatar filename is absent

#### Scenario: Videos of a playlist excluded from home are left out
- **WHEN** a downloaded video that would qualify for any section is tracked only by a playlist excluded from home
- **THEN** it appears in no section

#### Scenario: A video also tracked by a source shown on home still appears
- **WHEN** a downloaded video is tracked by a playlist excluded from home and by a channel or playlist that is not
- **THEN** it appears as usual, with the source that is not excluded as its source

#### Scenario: Excluded videos leave room for others
- **WHEN** a playlist excluded from home holds more recently synced videos than a playlist shown on home
- **THEN** "Latest videos" and "Quick watches" fill with the shown playlist's videos, as if the excluded playlist were not tracked

#### Scenario: Including a playlist in home again
- **WHEN** a playlist excluded from home is included in home again
- **THEN** its downloaded videos appear in the home sections they qualify for
