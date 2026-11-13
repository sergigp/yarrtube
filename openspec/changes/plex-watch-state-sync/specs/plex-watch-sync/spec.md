# Spec Delta

## Purpose

Keeps each downloaded video's watch state (watched, and where playback stopped) in sync with its Plex episode in both directions, so a video watched or started in either the web UI or Plex shows the same in the other.

## ADDED Requirements

### Requirement: Plex Watch Sync Is Optional
The system SHALL sync watch state with Plex only when `YARRTUBE_PLEX_URL`, `YARRTUBE_PLEX_TOKEN` and `YARRTUBE_PLEX_VIDEOS_PATH` are all set. Otherwise it MUST NOT contact any Plex server, and watch state keeps working in yarrtube unchanged.

#### Scenario: Integration not configured
- **WHEN** the daemon runs with any of the three variables unset and a video is marked watched
- **THEN** no request is made to a Plex server

### Requirement: One Shared Watch State
The system SHALL compare a video and its Plex episode as one of three states: unplayed, in progress at a position in whole seconds, or watched. A Plex episode SHALL count as watched when Plex reports it viewed at least once and holds no resume position. It SHALL count as in progress at its resume position, unless it is watched and the position is at or below 10% of its duration, in which case it SHALL count as watched. Otherwise it SHALL count as unplayed.

#### Scenario: Episode finished in Plex
- **WHEN** Plex reports an episode viewed once with no resume position
- **THEN** it counts as watched

#### Scenario: Episode started in Plex
- **WHEN** Plex reports an episode never finished, with a resume position of 3 minutes 35 seconds
- **THEN** it counts as in progress at 215 seconds

#### Scenario: Short rewatch in Plex
- **WHEN** Plex reports a watched 40-minute episode with a resume position of 2 minutes
- **THEN** it counts as watched

#### Scenario: Long rewatch in Plex
- **WHEN** Plex reports a watched 40-minute episode with a resume position of 10 minutes
- **THEN** it counts as in progress at 600 seconds

### Requirement: Changes Made In Plex Reach yarrtube
When an episode's state in Plex changes and the video's state in yarrtube does not, the next watch sync SHALL give the video the episode's state, on every stored copy of its YouTube video. Watched or in progress SHALL set the video's last played time to when Plex last played the episode.

#### Scenario: Watched in Plex
- **WHEN** an unwatched video's episode is watched in Plex
- **THEN** after the next watch sync the video is watched with a saved position of 0

#### Scenario: Started in Plex
- **WHEN** a video's episode is played in Plex up to 3 minutes 35 seconds and stopped
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
- **WHEN** a video is played in the web UI up to 5 minutes and stopped
- **THEN** after the next watch sync its Plex episode resumes at 5 minutes

#### Scenario: Rewatch in the web UI
- **WHEN** a watched video is rewatched in the web UI past 10% and stopped at 10 minutes, so it becomes unwatched
- **THEN** after the next watch sync its Plex episode is unwatched and resumes at 10 minutes

### Requirement: Every Copy's Episode Follows
When a change is taken from one episode, the other stored copies of the same YouTube video SHALL have their Plex episodes brought to the same state in the same watch sync.

#### Scenario: Watched in a channel's show
- **WHEN** a YouTube video tracked by a channel and a playlist is watched in Plex in the channel's show
- **THEN** after the next watch sync both copies are watched in yarrtube and the playlist's episode is watched in Plex

### Requirement: Most Recent Change Wins
When both the video's state in yarrtube and its episode's state in Plex changed since they last matched, and they differ, the side played or marked most recently SHALL win and be copied to the other. A side with no known time of change SHALL lose.

#### Scenario: Plex played later
- **WHEN** a video is stopped at 2 minutes in the web UI, and later its episode is watched in Plex, before a watch sync runs
- **THEN** after the next watch sync the video is watched

#### Scenario: Web UI played later
- **WHEN** an episode is watched in Plex, and later its video is stopped at 2 minutes in the web UI, before a watch sync runs
- **THEN** after the next watch sync the episode resumes at 2 minutes in Plex and is unwatched

### Requirement: Synced Changes Are Not Echoed
A change the sync copied to one side SHALL NOT be taken as a new change of that side by a later sync.

#### Scenario: Second sync after a push
- **WHEN** a watch sync marks an episode watched in Plex, and another watch sync runs with nothing changed since
- **THEN** the second sync sends no change to Plex and changes no video

### Requirement: First Sync Copies What Was Played
For a video whose state has never been synced with its episode, the sync SHALL treat both as having last matched as unplayed: a side that was played or marked is copied to an unplayed other side, and when both were played the most recent change wins.

#### Scenario: Upgrade with history on both sides
- **WHEN** the daemon starts after this change with an episode watched only in Plex and another video watched only in the web UI
- **THEN** after the first watch sync the first video is watched in yarrtube and the second video's episode is watched in Plex

#### Scenario: Neither side played
- **WHEN** a video and its episode were never played
- **THEN** the first watch sync changes neither

### Requirement: Episodes Not In Plex Yet
A video that is not downloaded, or whose episode Plex has not imported yet, SHALL be skipped without error and synced by a later watch sync once Plex lists its episode. When Plex re-imports a video's file as a new episode, the new episode SHALL be treated as never synced, so its empty state does not unwatch the video.

#### Scenario: Not imported yet
- **WHEN** a watch sync runs before Plex has imported a downloaded video
- **THEN** nothing is sent to Plex for it and no error is reported, and a later sync syncs it

#### Scenario: Episode re-imported
- **WHEN** Plex re-imports a watched video's file as a new, unplayed episode
- **THEN** the next watch sync marks the new episode watched and the video stays watched

### Requirement: Periodic Watch Sync
The system SHALL run a watch sync at startup and then every 5 minutes while the integration is configured, in the same task lane as the Plex label sync, so it never delays downloads. When more changes are waiting for Plex than one sync sends, the next sync SHALL run right after it.

#### Scenario: Next sync
- **WHEN** a watch sync finishes with nothing left to send
- **THEN** the next watch sync is scheduled 5 minutes later

#### Scenario: Large backlog
- **WHEN** marking a whole channel watched leaves more episodes to update than one sync sends
- **THEN** another watch sync runs right after it until they are all sent

### Requirement: Watch Sync Survives Plex Failures
When Plex cannot be reached, the watch sync SHALL change nothing and retry with the next periodic sync. When Plex rejects the change for one episode, the others in the same sync SHALL still be synced, and the rejected one SHALL be retried by a later sync.

#### Scenario: Plex down
- **WHEN** a watch sync runs while the Plex server is unreachable
- **THEN** no video changes, and the next watch sync is scheduled 5 minutes later

#### Scenario: One episode rejected
- **WHEN** Plex rejects the change for one of two episodes in a watch sync
- **THEN** the other episode is updated and the rejected one is retried by a later sync

### Requirement: Watch Sync In The Tasks View
The tasks view SHALL describe a pending or running watch sync as "Syncing Plex watch state", in the **Other** tab.

#### Scenario: Watch sync pending
- **WHEN** a watch sync is pending
- **THEN** the tasks view lists it as "Syncing Plex watch state" under **Other**
