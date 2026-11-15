## ADDED Requirements

### Requirement: Listed Videos Include Their Episode Number
The system SHALL include each video's episode number in the responses that list the videos of a playlist, of a channel and of a category: for a playlist video the `<n>` of its `S01E<n>` file name, for a channel video its stored `<MMDDii>` number. It SHALL report the number as absent for a video that has none assigned yet.

#### Scenario: Playlist video with an episode
- **WHEN** a client lists the videos of a playlist whose video is stored as episode 7
- **THEN** that video's entry includes the episode number 7

#### Scenario: Video without an episode
- **WHEN** a client lists the videos of a playlist or channel holding a video with no episode number assigned
- **THEN** that video's entry reports its episode number as absent
