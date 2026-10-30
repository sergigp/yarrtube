# Spec Delta

## MODIFIED Requirements

### Requirement: List Home Videos Across Sources
The system SHALL provide an HTTP endpoint that returns, in one response, the downloaded videos of the home sections across every tracked channel and every tracked playlist not excluded from home, so that a YouTube video appears in at most one section.

A playlist excluded from home SHALL NOT be the source of any video in any section. A video it tracks SHALL still appear when another source that is not excluded tracks it, through that source.

"Continue watching" SHALL hold up to 6 videos the user has started and not finished recently:
- each is unwatched, with a saved playback position greater than 30 seconds, and was last played within the past 7 days
- ordered by last played time, most recent first

"Quick watches" SHALL hold up to 6 short videos:
- each is unwatched, with a recorded duration shorter than 15 minutes (900 seconds), and not shown under "Continue watching" or any category section
- ordered by sync time (the time the video was recorded by the system), newest first

Category sections, described in "Home Category Sections", SHALL come after "Continue watching" and before "Quick watches".

In these sections, a YouTube video tracked by more than one source SHALL appear once, from a tracking channel when there is one.

"Latest videos" SHALL hold up to 18 videos that are not shown under "Continue watching", any category section or "Quick watches", ordered by sync time, newest first, once per source that tracks them.

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
- **THEN** the daemon responds with HTTP status 200, three empty sections and no category sections

#### Scenario: A started short video counts as continue watching
- **WHEN** a video qualifies for both "Continue watching" and "Quick watches"
- **THEN** it appears under "Continue watching" only

#### Scenario: A video shown above is not repeated in latest videos
- **WHEN** a video appears under "Continue watching", a category section or "Quick watches"
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

#### Scenario: A video shown in a category section is not a quick watch
- **WHEN** a short, unwatched video is shown in a category section
- **THEN** it does not appear under "Quick watches"

## ADDED Requirements

### Requirement: Home Category Sections
The home endpoint SHALL return up to 3 category sections, each with the category's identifier, name and color and up to 6 of its pending videos: downloaded, unwatched videos from the home sources that belong to the category and are not shown under "Continue watching" or an earlier category section, newest sync first. A category with no such videos SHALL get no section.

#### Scenario: Category with pending videos
- **WHEN** "Chess" has 2 downloaded, unwatched videos and no other category has any
- **THEN** the response holds one category section for "Chess", with its identifier, name and color, listing both videos, the more recently synced first

#### Scenario: Watched videos are not pending
- **WHEN** every downloaded video of "Chess" is watched
- **THEN** the response holds no category section for "Chess"

#### Scenario: Category section is bounded
- **WHEN** "Chess" has 10 pending videos
- **THEN** its section holds the 6 most recently synced

#### Scenario: Continue watching videos are left out
- **WHEN** a video of "Chess" is shown under "Continue watching"
- **THEN** it does not appear in the "Chess" section

#### Scenario: Video in two categories appears once
- **WHEN** a pending video belongs to "Chess" and "Streams", and both get a section
- **THEN** it appears only in the first of the two sections

#### Scenario: Excluded playlists do not feed category sections
- **WHEN** a pending video of "Highlights" is tracked only by a playlist excluded from home
- **THEN** it appears in no category section

#### Scenario: Video reached through a source shown on home
- **WHEN** a pending video belongs to "Highlights" through a playlist excluded from home and is also tracked by a channel that is not excluded
- **THEN** it can appear in the "Highlights" section, with the channel as its source

### Requirement: Home Category Section Ranking
Categories SHALL be ranked by their number of pending videos not shown under "Continue watching", counted in full and not only up to 6, most first, ties broken by category name ignoring case. Sections SHALL follow the ranking; a category left without videos once those shown in earlier sections are removed SHALL be skipped and the next ranked category SHALL take its place, until 3 sections are filled or no category remains.

#### Scenario: Most pending first
- **WHEN** "Chess" has 4 pending videos, "Science" has 9 and "Streams" has 1
- **THEN** the sections are, in order, "Science", "Chess" and "Streams"

#### Scenario: At most three sections
- **WHEN** 5 categories have pending videos
- **THEN** only the 3 with the most pending videos get a section

#### Scenario: Ties by name
- **WHEN** "chess" and "Astronomy" both have 3 pending videos
- **THEN** "Astronomy" comes before "chess"

#### Scenario: Emptied category is skipped
- **WHEN** every pending video of the second-ranked category is shown in the first-ranked section, and a fourth category has pending videos
- **THEN** the second-ranked category gets no section and the fourth one does
