# Spec Delta

## ADDED Requirements

### Requirement: Mark A Playlist Watched
The system SHALL provide an HTTP endpoint that, given a tracked playlist's ID, marks every downloaded video of that playlist as watched, including every other stored copy of those videos, such as their copies in channels. Videos in the playlist that have not finished downloading SHALL be left unchanged. The endpoint SHALL respond with HTTP status 204.

#### Scenario: Marking a playlist watched
- **WHEN** a client marks a tracked playlist as watched
- **THEN** the daemon responds with HTTP status 204, every downloaded video of the playlist is watched, and its unwatched count becomes 0

#### Scenario: Copies in channels are marked too
- **WHEN** a client marks a playlist as watched and one of its downloaded videos is also tracked by a channel
- **THEN** the channel's copy is watched too, and the channel's unwatched count drops accordingly

#### Scenario: Playlist videos still downloading are left unchanged
- **WHEN** a client marks a playlist as watched while some of its videos are pending or downloading
- **THEN** those videos stay unwatched

#### Scenario: Playlist does not exist
- **WHEN** a client marks as watched a playlist ID that is not currently tracked
- **THEN** the daemon responds with HTTP status 400 and changes nothing

#### Scenario: Invalid playlist ID
- **WHEN** a client marks as watched a value that is not a valid playlist ID
- **THEN** the daemon responds with HTTP status 400 and changes nothing

## MODIFIED Requirements

### Requirement: Mark A Video Watched
The system SHALL provide an HTTP endpoint that, given a YouTube video ID, marks every stored copy of that video as watched, with its saved position reset to 0. A video SHALL be marked only when at least one of its stored copies has finished downloading. A video that is already watched SHALL be left unchanged. On success the endpoint SHALL respond with HTTP status 204.

#### Scenario: Marking a video watched
- **WHEN** a client marks a downloaded, unwatched video as watched
- **THEN** the daemon responds with HTTP status 204, the video is watched and its saved position is 0

#### Scenario: Every stored copy is marked
- **WHEN** a client marks as watched a video tracked by both a channel and a playlist
- **THEN** both stored copies are watched, and both the channel's and the playlist's unwatched counts drop by one

#### Scenario: Marking drops the video from continue watching
- **WHEN** a client marks as watched a video listed under "Continue watching"
- **THEN** the video no longer qualifies for "Continue watching", and its channel's unwatched count drops by one

#### Scenario: Already watched video
- **WHEN** a client marks as watched a video that is already watched
- **THEN** the daemon responds with HTTP status 204 and the video's watched time is unchanged

#### Scenario: Unknown video
- **WHEN** a client marks as watched a YouTube video ID that no playlist or channel tracks
- **THEN** the daemon responds with HTTP status 400 and changes nothing

#### Scenario: Video not downloaded
- **WHEN** a client marks as watched a video none of whose stored copies has finished downloading
- **THEN** the daemon responds with HTTP status 400 and changes nothing
