## MODIFIED Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a YouTube playlist ID, a name, a storage path, and a quality tier (`high`, `mid`, or `low`), confirms the YouTube playlist ID corresponds to an existing, accessible YouTube playlist before persisting, and stores the playlist with kind `youtube_linked`, that path, quality, and a system-generated creation timestamp.

The storage path identifies where the playlist's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `music/chill`).

A playlist created through this endpoint always has kind `youtube_linked`; see `custom-playlist-crud` for creating a playlist with no YouTube playlist behind it.

#### Scenario: Successful creation
- **WHEN** a request supplies a YouTube playlist ID that exists on YouTube, a valid name, a valid path, and a valid quality
- **THEN** the system persists a playlist record with that ID, name, path, quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its kind

#### Scenario: Duplicate playlist ID
- **WHEN** a request supplies a YouTube playlist ID that already exists in storage
- **THEN** the system accepts the request but makes no change bc the endpoint is idempotent, and returns the existing playlist record

#### Scenario: Duplicate playlist ID with a different quality
- **WHEN** a request supplies a YouTube playlist ID that already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Duplicate playlist ID with a different path
- **WHEN** a request supplies a YouTube playlist ID that already exists in storage, with a path value different from the stored record's
- **THEN** the system makes no change, ignores the request's path value, and returns the existing record with its original path

#### Scenario: Invalid name
- **WHEN** a request supplies an empty name, or a name containing characters that are unsafe for a filesystem folder name
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid path
- **WHEN** a request omits path, supplies an empty path, or supplies a path that is absolute, contains a `..` segment, or contains an empty segment (e.g. leading/trailing/doubled `/`)
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid quality
- **WHEN** a request omits quality, or supplies a value that is not `high`, `mid`, or `low`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube playlist
- **WHEN** a request supplies a YouTube playlist ID that does not correspond to an existing, accessible YouTube playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

### Requirement: Delete Playlist
The system SHALL provide an HTTP endpoint that deletes a previously created playlist identified by its ID, regardless of the playlist's kind.

#### Scenario: Successful deletion
- **WHEN** a request identifies a playlist ID that exists in storage
- **THEN** the system removes it and confirms the deletion

#### Scenario: Deleting a custom playlist
- **WHEN** a request identifies a playlist ID that exists in storage with kind `custom`
- **THEN** the system removes it the same way it would a YouTube-linked playlist

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system reports that nothing was found and makes no change to storage and returns a bad request with a meaningful error description

### Requirement: List Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist, of either kind.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, path, quality, kind, and creation timestamp

#### Scenario: Playlists of both kinds exist
- **WHEN** at least one YouTube-linked playlist and at least one custom playlist have been created
- **THEN** the system returns both together in a single list, each item's kind distinguishing them

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error

### Requirement: Playlist Deletion Publishes a Domain Event
The system SHALL publish a PlaylistDeleted domain event, containing the playlist's ID, whenever a playlist is successfully deleted, regardless of its kind.

#### Scenario: Existing playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID

#### Scenario: Existing custom playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist with kind `custom` from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID, the same way it would for a YouTube-linked playlist

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a delete-playlist request identifies a playlist ID that does not exist in storage
- **THEN** the system does not publish a PlaylistDeleted event
