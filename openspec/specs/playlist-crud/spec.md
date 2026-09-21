## Purpose

Lets a caller create, delete, and list the playlists the daemon tracks, so a future scheduler has a persisted set of playlists to query and download from.

## Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, a name, a storage path, and a quality tier (`high`, `mid`, or `low`). The system SHALL extract the playlist ID from the `playlist` value when it is a URL, confirms the resulting YouTube playlist ID corresponds to an existing, accessible YouTube playlist before persisting, and stores the playlist with kind `youtube_linked`, that ID, path, quality, and a system-generated creation timestamp.

The storage path identifies where the playlist's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `music/chill`).

Every playlist created through this endpoint has kind `youtube_linked`.

#### Scenario: Successful creation
- **WHEN** a request supplies a `playlist` value that is a bare YouTube playlist ID that exists on YouTube, a valid name, a valid path, and a valid quality
- **THEN** the system persists a playlist record with that ID, name, path, quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its kind

#### Scenario: Successful creation from a YouTube URL
- **WHEN** a request supplies a `playlist` value that is a full YouTube playlist URL (e.g. `https://www.youtube.com/playlist?list=PLabc123`) whose extracted playlist ID exists on YouTube, a valid name, a valid path, and a valid quality
- **THEN** the system extracts the playlist ID from the URL, persists a playlist record with that ID, name, path, quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its kind

#### Scenario: Duplicate playlist ID
- **WHEN** a request supplies a `playlist` value (bare ID or URL) whose playlist ID already exists in storage
- **THEN** the system accepts the request but makes no change bc the endpoint is idempotent, and returns the existing playlist record

#### Scenario: Duplicate playlist ID with a different quality
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Duplicate playlist ID with a different path
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a path value different from the stored record's
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

#### Scenario: Missing or empty playlist value
- **WHEN** a request omits the `playlist` field, or supplies an empty or whitespace-only value
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Unrecognized URL
- **WHEN** a request supplies a `playlist` value that is a URL but not a recognized YouTube URL
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: YouTube URL missing the playlist parameter
- **WHEN** a request supplies a `playlist` value that is a recognized YouTube URL (e.g. a video watch URL) that has no `list` query parameter
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube playlist
- **WHEN** a request supplies a `playlist` value whose extracted playlist ID does not correspond to an existing, accessible YouTube playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

### Requirement: Unique Playlist Paths
The system SHALL reject a request to create a playlist whose storage path is already used by a different existing playlist.

#### Scenario: Path already used by another playlist
- **WHEN** a create request supplies a path that is already the stored path of a different existing playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Re-submitting the same playlist's own path
- **WHEN** a create request identifies a playlist ID that already exists and supplies that same playlist's own stored path
- **THEN** the system treats this as the existing idempotent duplicate-ID case and does not reject it under this rule

### Requirement: Delete a Playlist
The system SHALL provide an HTTP endpoint that deletes a previously created playlist identified by its ID, and SHALL delete every video record stored for that playlist as part of the same operation, so that no video record can ever be observed referencing a playlist that no longer exists.

#### Scenario: Successful deletion
- **WHEN** a request identifies a playlist ID that exists in storage
- **THEN** the system removes it, removes every video record stored for it, and confirms the deletion

#### Scenario: Deleting a playlist with videos
- **WHEN** a request identifies a playlist ID that has one or more video records stored for it, regardless of their download status
- **THEN** by the time the system confirms the deletion, none of those video records remain in storage

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system reports that nothing was found and makes no change to storage, including no change to any video records, and returns a bad request with a meaningful error description

### Requirement: List All Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, path, quality, kind, and creation timestamp

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error

### Requirement: Playlist Creation Publishes a Domain Event
The system SHALL publish a PlaylistCreated domain event, containing the playlist's ID, whenever a playlist is newly created — and SHALL NOT publish it when creation is a no-op because the playlist already existed.

#### Scenario: New playlist created
- **WHEN** a create-playlist request results in a new playlist being persisted
- **THEN** the system publishes a PlaylistCreated event containing that playlist's ID

#### Scenario: Playlist already existed
- **WHEN** a create-playlist request identifies a playlist ID that already exists in storage
- **THEN** the system does not publish a PlaylistCreated event

### Requirement: Playlist Deletion Publishes an Event
The system SHALL publish a PlaylistDeleted domain event, containing the playlist's ID and its storage path, whenever a playlist is successfully deleted. The path travels with the event because the playlist record itself no longer exists by the time anything reacts to it.

#### Scenario: Existing playlist deleted
- **WHEN** a delete-playlist request successfully removes a playlist from storage
- **THEN** the system publishes a PlaylistDeleted event containing that playlist's ID and its storage path

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a delete-playlist request identifies a playlist ID that does not exist in storage
- **THEN** the system does not publish a PlaylistDeleted event
