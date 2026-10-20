## ADDED Requirements

### Requirement: List Videos For A Category
The system SHALL provide an HTTP endpoint that, given a category's identifier, returns the downloaded videos of all the category's members, each YouTube video once, ordered by sync time (the time the video was recorded by the system), newest first. When a YouTube video is tracked by more than one member, it SHALL be returned from a tracking channel when there is one. Playlists excluded from home SHALL still contribute their videos. Each returned video SHALL include the same fields as a video of the home sections, including the source that tracks it. When no category has the identifier, the endpoint SHALL respond with HTTP status 404.

#### Scenario: Videos across members
- **WHEN** the category "Chess" has a channel member with 2 downloaded videos and a playlist member with 1 downloaded video
- **THEN** the daemon responds with HTTP status 200 and the 3 videos, newest sync first, each with its source

#### Scenario: A video tracked by two members
- **WHEN** a YouTube video is tracked by both a channel member and a playlist member of the category
- **THEN** it is returned once, from the channel

#### Scenario: Only downloaded videos
- **WHEN** a member of the category has videos that are pending, downloading, failed or excluded
- **THEN** those videos are not returned

#### Scenario: Playlist excluded from home
- **WHEN** a playlist member of the category is excluded from home
- **THEN** its downloaded videos are still returned

#### Scenario: Category without members
- **WHEN** the category has no members
- **THEN** the daemon responds with HTTP status 200 and no videos

#### Scenario: Unknown category
- **WHEN** a client lists the videos of a category identifier that does not exist
- **THEN** the daemon responds with HTTP status 404
