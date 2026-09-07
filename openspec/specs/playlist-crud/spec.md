## Purpose

Lets a caller create, delete, and list the playlists the daemon tracks, so a future scheduler has a persisted set of playlists to query and download from.

## Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a YouTube playlist ID and a name, confirms the YouTube playlist ID corresponds to an existing, accessible YouTube playlist before persisting, and stores the playlist with a system-generated creation timestamp.

#### Scenario: Successful creation
- **WHEN** a request supplies a YouTube playlist ID that exists on YouTube and a valid name
- **THEN** the system persists a playlist record with that ID, name, and a creation timestamp, and returns the created playlist

#### Scenario: Duplicate playlist ID
- **WHEN** a request supplies a YouTube playlist ID that already exists in storage
- **THEN** the system accepts the request but makes no change bc the endpoint is idempotent, and returns the existing playlist record

#### Scenario: Invalid name
- **WHEN** a request supplies an empty name, or a name containing characters that are unsafe for a filesystem folder name
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube playlist
- **WHEN** a request supplies a YouTube playlist ID that does not correspond to an existing, accessible YouTube playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

### Requirement: Delete Playlist
The system SHALL provide an HTTP endpoint that deletes a previously created playlist identified by its YouTube playlist ID.

#### Scenario: Successful deletion
- **WHEN** a request identifies a playlist ID that exists in storage
- **THEN** the system removes it and confirms the deletion

#### Scenario: Deleting a nonexistent playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system reports that nothing was found and makes no change to storage and returns a bad request with a meaningful error description

### Requirement: List Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, and creation timestamp

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error
