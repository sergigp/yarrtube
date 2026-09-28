## ADDED Requirements

### Requirement: Listed Videos Include Their Metadata
The system SHALL include each video's publish time, description and channel name, taken from its generated metadata, in the responses that list the videos of a playlist and of a channel. The description SHALL be the same truncated plot recorded for the video's metadata file. For a video whose metadata has not been generated, the system SHALL report all three as absent.

#### Scenario: Video with generated metadata is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video with generated metadata
- **THEN** that video's entry includes its publish time, its description and the name of the channel that published it

#### Scenario: Video without generated metadata is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video whose metadata has not been generated
- **THEN** that video's entry reports its publish time, description and channel name as absent, and every other field is still returned
