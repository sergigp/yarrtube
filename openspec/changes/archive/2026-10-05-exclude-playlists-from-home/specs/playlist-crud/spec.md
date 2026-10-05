## MODIFIED Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, a storage path, a quality tier (`high`, `mid`, or `low`), and an optional `exclude_from_home` boolean that defaults to `false` when omitted. The request SHALL NOT carry a name. The system SHALL extract the playlist ID from the `playlist` value when it is a URL, and SHALL look the resulting YouTube playlist ID up on YouTube before persisting, confirming that it corresponds to an existing, accessible YouTube playlist. It SHALL store the playlist with kind `youtube_linked`, that ID, the playlist's YouTube title as its name, the path, the quality, the `exclude_from_home` setting, and a system-generated creation timestamp. When YouTube reports an empty or blank title, the system SHALL use the playlist ID as the name.

The storage path identifies where the playlist's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `music/chill`).

The name is a display name only and never names a directory. It SHALL be accepted whatever characters it contains, including `/`, `:` and `?`, as long as it is not blank.

Every playlist created through this endpoint has kind `youtube_linked`.

#### Scenario: Successful creation
- **WHEN** a request supplies a `playlist` value that is a bare YouTube playlist ID that exists on YouTube with the title "Lofi beats", a valid path, and a valid quality
- **THEN** the system persists a playlist record with that ID, the name "Lofi beats", that path and quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its name and kind

#### Scenario: Successful creation from a YouTube URL
- **WHEN** a request supplies a `playlist` value that is a full YouTube playlist URL (e.g. `https://www.youtube.com/playlist?list=PLabc123`) whose extracted playlist ID exists on YouTube, a valid path, and a valid quality
- **THEN** the system extracts the playlist ID from the URL, persists a playlist record with that ID, its YouTube title as the name, that path and quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its kind

#### Scenario: Created playlist shown on home by default
- **WHEN** a valid creation request omits `exclude_from_home`
- **THEN** the system persists the playlist with `exclude_from_home` set to `false`, and returns it with `exclude_from_home` `false`

#### Scenario: Creating a playlist excluded from home
- **WHEN** a valid creation request supplies `exclude_from_home` `true`
- **THEN** the system persists the playlist with `exclude_from_home` set to `true`, and returns it with `exclude_from_home` `true`

#### Scenario: Invalid exclude from home value
- **WHEN** a request supplies an `exclude_from_home` value that is not a boolean
- **THEN** the system rejects the request with a client error, without persisting anything and without checking YouTube

#### Scenario: YouTube title with filesystem-unsafe characters
- **WHEN** a request identifies a YouTube playlist whose title is "AC/DC: greatest hits?"
- **THEN** the system persists the playlist with the name "AC/DC: greatest hits?", and lists it under that name afterwards

#### Scenario: YouTube playlist with a blank title
- **WHEN** a request identifies a YouTube playlist whose title YouTube reports as empty
- **THEN** the system persists the playlist with its playlist ID as the name

#### Scenario: Invalid name
- **WHEN** a request supplies a `name` field, whether empty, containing filesystem-unsafe characters, or valid
- **THEN** the system neither rejects the request because of it nor uses it, and names the playlist after its YouTube title

#### Scenario: Duplicate playlist ID
- **WHEN** a request supplies a `playlist` value (bare ID or URL) whose playlist ID already exists in storage
- **THEN** the system accepts the request but makes no change bc the endpoint is idempotent, and returns the existing playlist record

#### Scenario: Duplicate playlist ID with a different quality
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Duplicate playlist ID with a different path
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with a path value different from the stored record's
- **THEN** the system makes no change, ignores the request's path value, and returns the existing record with its original path

#### Scenario: Duplicate playlist ID with a different exclude from home value
- **WHEN** a request supplies a `playlist` value whose playlist ID already exists in storage, with an `exclude_from_home` value different from the stored record's
- **THEN** the system makes no change, ignores the request's `exclude_from_home` value, and returns the existing record with its original setting

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

### Requirement: List All Playlists
The system SHALL provide an HTTP endpoint that returns every currently stored playlist, sorted alphabetically by name, ignoring case.

#### Scenario: Playlists exist
- **WHEN** one or more playlists have been created
- **THEN** the system returns all of them, each with its ID, name, path, quality, kind, whether it is excluded from home, and creation timestamp

#### Scenario: No playlists exist
- **WHEN** no playlists have been created
- **THEN** the system returns an empty list rather than an error

#### Scenario: Playlists are sorted by name regardless of creation order
- **WHEN** playlists named "watch later", "Courses" and "Ambient" were created in that order
- **THEN** the system returns them in the order "Ambient", "Courses", "watch later"

## ADDED Requirements

### Requirement: Change Whether A Playlist Is Excluded From Home
The system SHALL provide an HTTP endpoint that changes whether a previously created playlist, identified by its ID, is excluded from home, given a required `exclude_from_home` boolean. It SHALL change only that setting, leaving the playlist's other fields and its video records as they were, and SHALL return the updated playlist. Setting the value the playlist already has SHALL succeed without changing anything.

#### Scenario: Excluding a playlist from home
- **WHEN** a request sets `exclude_from_home` to `true` on a playlist that is shown on home
- **THEN** the system stores the playlist as excluded from home, returns it with `exclude_from_home` `true`, and leaves its name, path, quality, kind, creation timestamp and video records unchanged

#### Scenario: Including a playlist in home again
- **WHEN** a request sets `exclude_from_home` to `false` on a playlist that is excluded from home
- **THEN** the system stores the playlist as shown on home, and returns it with `exclude_from_home` `false`

#### Scenario: Setting the value the playlist already has
- **WHEN** a request sets `exclude_from_home` to the value the playlist already has
- **THEN** the system succeeds and returns the playlist unchanged

#### Scenario: Unknown playlist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system makes no change and responds with not found and a meaningful error description

#### Scenario: Missing exclude from home value
- **WHEN** a request omits `exclude_from_home`
- **THEN** the system makes no change and returns a bad request with a meaningful error description

#### Scenario: Invalid exclude from home value
- **WHEN** a request supplies an `exclude_from_home` value that is not a boolean
- **THEN** the system makes no change and rejects the request with a client error
