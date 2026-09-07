## MODIFIED Requirements

### Requirement: CLI Invocation
The system SHALL provide a command that accepts exactly two positional arguments: a YouTube playlist ID and a local output directory path.

#### Scenario: Valid invocation
- **WHEN** the user runs the command with a playlist ID and an output path
- **THEN** the system proceeds to resolve the playlist and begin downloading

#### Scenario: Missing arguments
- **WHEN** the user runs the command with fewer than two arguments
- **THEN** the system prints a usage error and exits with a non-zero status without making any network calls

### Requirement: Playlist Resolution
The system SHALL resolve a playlist ID into an ordered list of member videos (URL and title) by querying the YouTube Data API v3 `playlistItems` resource, following pagination until all pages are retrieved.

#### Scenario: Single-page playlist
- **WHEN** the playlist has 50 or fewer videos
- **THEN** the system retrieves the full list in one API request and preserves the playlist's ordering

#### Scenario: Multi-page playlist
- **WHEN** the playlist has more than 50 videos
- **THEN** the system follows the API's page token across multiple requests and combines all pages into a single ordered list before starting any downloads

#### Scenario: Invalid or inaccessible playlist
- **WHEN** the playlist ID does not correspond to an existing, accessible playlist, or the API returns an error
- **THEN** the system prints the error and exits with a non-zero status without invoking `yt-dlp`

#### Scenario: Empty playlist
- **WHEN** the playlist exists but contains zero videos
- **THEN** the system reports that no videos were found and exits successfully without invoking `yt-dlp`
