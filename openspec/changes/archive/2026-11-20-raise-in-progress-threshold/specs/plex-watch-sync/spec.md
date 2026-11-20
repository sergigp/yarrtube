## MODIFIED Requirements

### Requirement: One Shared Watch State
The system SHALL compare a video and its Plex episode as one of three states: unplayed, in progress at a position in whole seconds, or watched. A Plex episode SHALL count as watched when Plex reports it viewed at least once and holds no resume position. It SHALL count as in progress at its resume position when that position counts as started (see `watch-state`'s "Started Playback Position", against the episode's duration). A resume position that does not count as started SHALL be ignored: the episode SHALL count as watched when Plex reports it viewed at least once, and as unplayed otherwise. Otherwise it SHALL count as unplayed.

#### Scenario: Episode finished in Plex
- **WHEN** Plex reports an episode viewed once with no resume position
- **THEN** it counts as watched

#### Scenario: Episode started in Plex
- **WHEN** Plex reports a 15-minute episode never finished, with a resume position of 3 minutes 35 seconds
- **THEN** it counts as in progress at 215 seconds

#### Scenario: Episode barely started in Plex
- **WHEN** Plex reports a 40-minute episode never finished, with a resume position of 5 minutes
- **THEN** it counts as unplayed

#### Scenario: Short rewatch in Plex
- **WHEN** Plex reports a watched 40-minute episode with a resume position of 2 minutes
- **THEN** it counts as watched

#### Scenario: Long rewatch in Plex
- **WHEN** Plex reports a watched 40-minute episode with a resume position of 10 minutes
- **THEN** it counts as in progress at 600 seconds

### Requirement: Short Positions Count As Unplayed
On both sides, a position of an unwatched video that does not count as started (see `watch-state`'s "Started Playback Position") SHALL count as unplayed, so a video only sampled is never offered to resume elsewhere. When the newer side holds such a position, the other side SHALL be brought to unplayed, which can clear a resume position held there.

#### Scenario: Sampled for a few seconds in the web UI
- **WHEN** an unwatched video is stopped at 12 seconds in the web UI and its episode is unplayed in Plex
- **THEN** the watch sync sends nothing to Plex for it

#### Scenario: Opened briefly in Plex
- **WHEN** an unplayed video's episode is stopped at 20 seconds in Plex
- **THEN** the watch sync leaves the video unplayed

#### Scenario: Sampled for a share of a video in the web UI
- **WHEN** an unwatched 40-minute video is stopped at 5 minutes in the web UI and its episode is unplayed in Plex
- **THEN** the watch sync sends nothing to Plex for it

#### Scenario: Played for 20% in the web UI
- **WHEN** an unwatched 40-minute video is stopped at 8 minutes in the web UI and its episode is unplayed in Plex
- **THEN** after the next watch sync its Plex episode resumes at 8 minutes

### Requirement: Changes Made In Plex Reach yarrtube
When an episode's state in Plex changes and the video's state in yarrtube does not, the next watch sync SHALL give the video the episode's state, on every stored copy of its YouTube video. Watched or in progress SHALL set the video's last played time to when Plex last played the episode.

#### Scenario: Watched in Plex
- **WHEN** an unwatched video's episode is watched in Plex
- **THEN** after the next watch sync the video is watched with a saved position of 0

#### Scenario: Started in Plex
- **WHEN** a 15-minute video's episode is played in Plex up to 3 minutes 35 seconds and stopped
- **THEN** after the next watch sync the video is unwatched at 215 seconds, and is offered under "Continue watching"

#### Scenario: Unwatched in Plex
- **WHEN** a watched video's episode is marked unwatched in Plex
- **THEN** after the next watch sync the video is unwatched with a saved position of 0

### Requirement: Changes Made In yarrtube Reach Plex
When a video's state in yarrtube changes and its episode's state in Plex does not, the next watch sync SHALL give the episode the video's state: watched, unwatched at the saved position, or unwatched and unplayed.

#### Scenario: Watched in the web UI
- **WHEN** a video is marked watched in the web UI
- **THEN** after the next watch sync its Plex episode is watched

#### Scenario: Position in the web UI
- **WHEN** a 20-minute video is played in the web UI up to 5 minutes and stopped
- **THEN** after the next watch sync its Plex episode resumes at 5 minutes

#### Scenario: Rewatch in the web UI
- **WHEN** a watched 40-minute video is rewatched in the web UI and stopped at 10 minutes, so it becomes unwatched
- **THEN** after the next watch sync its Plex episode is unwatched and resumes at 10 minutes
