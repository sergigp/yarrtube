## MODIFIED Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, a name, a storage path, and a quality tier (`high`, `mid`, or `low`). The system SHALL extract the playlist ID from the `playlist` value when it is a URL, confirms the resulting YouTube playlist ID corresponds to an existing, accessible YouTube playlist before persisting, and stores the playlist with kind `youtube_linked`, that ID, path, quality, and a system-generated creation timestamp.

The storage path identifies where the playlist's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `music/chill`).

A playlist created through this endpoint always has kind `youtube_linked`; see `custom-playlist-crud` for creating a playlist with no YouTube playlist behind it.

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
