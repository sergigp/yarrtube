## ADDED Requirements

### Requirement: Playlist Creation Publishes a Domain Event
The system SHALL publish a PlaylistCreated domain event, containing the playlist's ID, whenever a playlist is newly created — and SHALL NOT publish it when creation is a no-op because the playlist already existed.

#### Scenario: New playlist created
- **WHEN** a create-playlist request results in a new playlist being persisted
- **THEN** the system publishes a PlaylistCreated event containing that playlist's ID

#### Scenario: Playlist already existed
- **WHEN** a create-playlist request identifies a playlist ID that already exists in storage
- **THEN** the system does not publish a PlaylistCreated event

### Requirement: Playlist Deletion Publishes a Domain Event
The system SHALL publish a PlaylistDeleted domain event, containing the playlist's ID, whenever a playlist is successfully deleted.

#### Scenario: Existing playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a delete-playlist request identifies a playlist ID that does not exist in storage
- **THEN** the system does not publish a PlaylistDeleted event
