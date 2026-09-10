## MODIFIED Requirements

### Requirement: Removed Video Cleanup
The system SHALL delete a stored video's record when a sync finds that it is no longer present in the source playlist, SHALL log each such deletion, and SHALL publish a `VideoDeleted` event carrying that video's title and whether it had been downloaded.

#### Scenario: Previously stored video no longer in playlist
- **WHEN** a sync finds that a video previously stored for a playlist is no longer a member of that playlist
- **THEN** the system deletes its stored record and logs the deletion

#### Scenario: Deletion publishes a VideoDeleted event
- **WHEN** a sync deletes a stored video's record because it is no longer in the source playlist
- **THEN** the system publishes a `VideoDeleted` event containing the playlist ID, the video ID, the video's title, and whether the video had been downloaded
