## ADDED Requirements

### Requirement: Only Watchable Items Count As Playlist Members (YouTube-Linked Playlists)
The system SHALL treat an item of the source YouTube playlist as a member of
the playlist only when YouTube reports its privacy status as public or
unlisted. Items reported as private, items with no privacy status (such as
deleted videos), and items with any other privacy status SHALL be ignored by
the membership diff, as if they were not in the playlist at all.

#### Scenario: Playlist contains a private video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a private item that has no stored record for that playlist
- **THEN** the system does not store it, does not fetch its thumbnail, and does not publish a `VideoAddedToPlaylist` event for it

#### Scenario: Playlist contains a deleted video
- **WHEN** a reconcile pass for a YouTube-linked playlist finds an item with no privacy status
- **THEN** the system ignores it the same way as a private item

#### Scenario: Playlist contains an unlisted video
- **WHEN** a reconcile pass for a YouTube-linked playlist finds an unlisted item that has no stored record for that playlist
- **THEN** the system stores it with status PENDING, the same as a public item

#### Scenario: Stored video that is not downloaded becomes private
- **WHEN** a reconcile pass for a YouTube-linked playlist finds that a stored video that is not downloaded is now private on YouTube
- **THEN** the system deletes its stored record and publishes a `VideoRemovedFromPlaylist` event, the same as for a video no longer in the playlist

#### Scenario: Downloaded video becomes private
- **WHEN** a reconcile pass for a YouTube-linked playlist finds that a downloaded video is now private on YouTube
- **THEN** the system deletes its stored record and publishes a `VideoRemovedFromPlaylist` event marking it as downloaded, so its local file is removed
