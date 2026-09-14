## MODIFIED Requirements

### Requirement: New Video Persistence (YouTube-Linked Playlists)
The system SHALL store each video found in a YouTube-linked playlist that is
not already stored for that playlist, with a status of PENDING and its
current position in the source YouTube playlist, and SHALL publish a
`VideoAdded` event for it.

#### Scenario: Playlist contains a video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video in the playlist that has no existing stored record for that playlist
- **THEN** the system stores it with status PENDING and its current playlist position, and publishes a `VideoAdded` event

#### Scenario: Playlist contains a video already stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video that already has a stored record for that playlist
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAdded` event

## ADDED Requirements

### Requirement: Video Position Tracking (YouTube-Linked Playlists)
The system SHALL update a stored video's recorded position to match its
current position in the source YouTube playlist on every reconcile pass,
regardless of whether the video is newly discovered or already stored,
so that a playlist reordered on YouTube is reflected the next time it is
reconciled.

#### Scenario: Video's position changed on YouTube since the last reconcile
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a stored video whose position in the source YouTube playlist differs from its previously recorded position
- **THEN** the system updates the stored position to match

#### Scenario: Video's position unchanged since the last reconcile
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a stored video whose position in the source YouTube playlist matches its previously recorded position
- **THEN** the system leaves the stored position unchanged
