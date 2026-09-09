## MODIFIED Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a YouTube playlist ID, a name, and a quality tier (`high`, `mid`, or `low`), confirms the YouTube playlist ID corresponds to an existing, accessible YouTube playlist before persisting, and stores the playlist with that quality and a system-generated creation timestamp.

#### Scenario: Successful creation
- **WHEN** a request supplies a YouTube playlist ID that exists on YouTube, a valid name, and a valid quality
- **THEN** the system persists a playlist record with that ID, name, quality, and a creation timestamp, and returns the created playlist

#### Scenario: Duplicate playlist ID
- **WHEN** a request supplies a YouTube playlist ID that already exists in storage
- **THEN** the system accepts the request but makes no change bc the endpoint is idempotent, and returns the existing playlist record

#### Scenario: Duplicate playlist ID with a different quality
- **WHEN** a request supplies a YouTube playlist ID that already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Invalid name
- **WHEN** a request supplies an empty name, or a name containing characters that are unsafe for a filesystem folder name
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid quality
- **WHEN** a request omits quality, or supplies a value that is not `high`, `mid`, or `low`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube playlist
- **WHEN** a request supplies a YouTube playlist ID that does not correspond to an existing, accessible YouTube playlist
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

### Requirement: List Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, quality, and creation timestamp

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error
