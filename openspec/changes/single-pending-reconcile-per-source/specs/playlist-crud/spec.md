## MODIFIED Requirements

### Requirement: Delete a Playlist
The system SHALL provide an HTTP endpoint that deletes a previously created playlist identified by its ID, and SHALL delete every video record stored for that playlist as part of the same operation, so that no video record can ever be observed referencing a playlist that no longer exists. Deleting a playlist SHALL also remove its pending recurring reconcile, so no reconcile of the playlist stays queued after it is gone.

#### Scenario: Successful deletion
- **WHEN** a request identifies a playlist ID that exists in storage
- **THEN** the system removes it, removes every video record stored for it, and confirms the deletion

#### Scenario: Deleting a playlist with videos
- **WHEN** a request identifies a playlist ID that has one or more video records stored for it, regardless of their download status
- **THEN** by the time the system confirms the deletion, none of those video records remain in storage

#### Scenario: Deleting a playlist with a pending reconcile
- **WHEN** a request deletes a playlist whose next recurring reconcile is pending
- **THEN** by the time the system confirms the deletion, no reconcile of that playlist is pending, and pending tasks of other playlists and channels are unchanged

#### Scenario: Playlist added again after deletion
- **WHEN** a playlist is deleted and then created again with the same ID
- **THEN** the playlist has a single recurring reconcile chain, as if it had never been tracked before

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system reports that nothing was found and makes no change to storage, including no change to any video records, and returns a bad request with a meaningful error description
