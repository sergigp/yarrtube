## Purpose

Lets a user download every video in a YouTube playlist to a local directory in one command, by resolving the playlist via the YouTube Data API v3 and delegating each download to `yt-dlp`.

## Requirements

### Requirement: CLI Invocation
The system SHALL provide a command that accepts exactly two positional arguments: a YouTube playlist ID and a local output directory path.

#### Scenario: Valid invocation
- **WHEN** the user runs the command with a playlist ID and an output path
- **THEN** the system proceeds to resolve the playlist and begin downloading

#### Scenario: Missing arguments
- **WHEN** the user runs the command with fewer than two arguments
- **THEN** the system prints a usage error and exits with a non-zero status without making any network calls

### Requirement: API Key Configuration
The system SHALL read a YouTube Data API v3 key from a `YOUTUBE_API_KEY` value in a `.env` file, and SHALL NOT hardcode or require the key on the command line.

#### Scenario: Key present
- **WHEN** `.env` defines `YOUTUBE_API_KEY` with a non-empty value
- **THEN** the system uses that value to authenticate requests to the YouTube Data API v3

#### Scenario: Key missing or empty
- **WHEN** `.env` is missing, or `YOUTUBE_API_KEY` is absent or empty
- **THEN** the system prints a clear error identifying the missing configuration and exits with a non-zero status before attempting any API call

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

### Requirement: Sequential Video Download
The system SHALL download each resolved video by invoking a locally installed `yt-dlp` executable, one video at a time in playlist order, saving output into the given output directory using the filename derived by the `video-naming` capability. The system SHALL NOT pass any format/quality selection flags in this version, relying on `yt-dlp`'s own default behavior for format and quality selection only.

#### Scenario: Multiple videos downloaded in order
- **WHEN** the resolved playlist contains multiple videos
- **THEN** the system invokes `yt-dlp` for the first video, waits for it to finish, then proceeds to the next video, never running two downloads concurrently

#### Scenario: Output directory does not exist
- **WHEN** the given output path does not already exist on disk
- **THEN** the system creates it before starting any downloads

### Requirement: Per-Video Failure Isolation
The system SHALL isolate failures to the individual video: if `yt-dlp` exits with a failure for one video, the system SHALL log the failure (including the video's title and URL) to the console and continue processing the remaining videos, and SHALL print a final summary of how many videos succeeded and which ones failed.

#### Scenario: A video in the middle of the playlist fails
- **WHEN** `yt-dlp` fails to download a video that is not the last one in the list
- **THEN** the system logs the failure and continues downloading the videos that follow it

#### Scenario: Run completes with some failures
- **WHEN** all videos in the playlist have been attempted and at least one failed
- **THEN** the system prints a summary listing the count of successful downloads and the titles/URLs of the failed ones, and exits with a non-zero status

#### Scenario: Run completes with no failures
- **WHEN** all videos in the playlist download successfully
- **THEN** the system prints a summary confirming the total count downloaded and exits with a zero status
