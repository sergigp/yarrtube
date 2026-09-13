## ADDED Requirements

### Requirement: Playlist Path Uniqueness
The system SHALL reject a request to create a playlist, of either kind, whose storage path is already used by a different existing playlist. This applies to both the create-playlist and create-custom-playlist endpoints, since they share the same storage path rules.

#### Scenario: Path already used by another playlist
- **WHEN** a create request supplies a path that is already the stored path of a different existing playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Path already used by a playlist of the other kind
- **WHEN** a create-custom-playlist request supplies a path that is already the stored path of an existing YouTube-linked playlist (or vice versa)
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Re-submitting the same playlist's own path
- **WHEN** a create request identifies a playlist ID that already exists and supplies that same playlist's own stored path
- **THEN** the system treats this as the existing idempotent duplicate-ID case and does not reject it under this rule

## MODIFIED Requirements

### Requirement: Delete Playlist
The system SHALL provide an HTTP endpoint that deletes a previously created playlist identified by its ID, regardless of the playlist's kind, and SHALL delete every video record stored for that playlist as part of the same operation, so that no video record can ever be observed referencing a playlist that no longer exists.

#### Scenario: Successful deletion
- **WHEN** a request identifies a playlist ID that exists in storage
- **THEN** the system removes it, removes every video record stored for it, and confirms the deletion

#### Scenario: Deleting a custom playlist
- **WHEN** a request identifies a playlist ID that exists in storage with kind `custom`
- **THEN** the system removes it and its video records the same way it would a YouTube-linked playlist

#### Scenario: Deleting a playlist with videos
- **WHEN** a request identifies a playlist ID that has one or more video records stored for it, regardless of their download status
- **THEN** by the time the system confirms the deletion, none of those video records remain in storage

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system reports that nothing was found and makes no change to storage, including no change to any video records, and returns a bad request with a meaningful error description

### Requirement: Playlist Deletion Publishes a Domain Event
The system SHALL publish a PlaylistDeleted domain event, containing the playlist's ID and its storage path, whenever a playlist is successfully deleted, regardless of its kind. The path travels with the event because the playlist record itself no longer exists by the time anything reacts to it.

#### Scenario: Existing playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID and its storage path

#### Scenario: Existing custom playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist with kind `custom` from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID and its storage path, the same way it would for a YouTube-linked playlist

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a delete-playlist request identifies a playlist ID that does not exist in storage
- **THEN** the system does not publish a PlaylistDeleted event
