## MODIFIED Requirements

### Requirement: New Video Persistence (YouTube-Linked Playlists)
The system SHALL store each video found in a YouTube-linked playlist that is
not already stored for that playlist, with a status of PENDING and its
current position in the source YouTube playlist, and SHALL publish a
`VideoAddedToPlaylist` event for it.

#### Scenario: Playlist contains a video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video in the playlist that has no existing stored record for that playlist
- **THEN** the system stores it with status PENDING and its current playlist position, and publishes a `VideoAddedToPlaylist` event

#### Scenario: Playlist contains a video already stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video that already has a stored record for that playlist
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAddedToPlaylist` event

### Requirement: Removed Video Cleanup (YouTube-Linked Playlists)
The system SHALL delete a stored video's record when a reconcile pass for a
YouTube-linked playlist finds that it is no longer present in the source
YouTube playlist, SHALL log each such deletion, and SHALL publish a
`VideoRemovedFromPlaylist` event carrying that video's title, its recorded
filename (if any), and whether it had been downloaded.

#### Scenario: Previously stored video no longer in the YouTube playlist
- **WHEN** a reconcile pass finds that a video previously stored for a YouTube-linked playlist is no longer a member of that playlist on YouTube
- **THEN** the system deletes its stored record and logs the deletion

#### Scenario: Deletion publishes a VideoDeleted event
- **WHEN** a reconcile pass deletes a stored video's record because it is no longer in the source YouTube playlist
- **THEN** the system publishes a `VideoRemovedFromPlaylist` event containing the playlist ID, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded
