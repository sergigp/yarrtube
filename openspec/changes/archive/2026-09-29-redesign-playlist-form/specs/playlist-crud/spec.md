## MODIFIED Requirements

### Requirement: Create Playlist
The system SHALL provide an HTTP endpoint that creates a playlist given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, a storage path, and a quality tier (`high`, `mid`, or `low`). The request SHALL NOT carry a name. The system SHALL extract the playlist ID from the `playlist` value when it is a URL, and SHALL look the resulting YouTube playlist ID up on YouTube before persisting, confirming that it corresponds to an existing, accessible YouTube playlist. It SHALL store the playlist with kind `youtube_linked`, that ID, the playlist's YouTube title as its name, the path, the quality, and a system-generated creation timestamp. When YouTube reports an empty or blank title, the system SHALL use the playlist ID as the name.

The storage path identifies where the playlist's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `music/chill`).

The name is a display name only and never names a directory. It SHALL be accepted whatever characters it contains, including `/`, `:` and `?`, as long as it is not blank.

Every playlist created through this endpoint has kind `youtube_linked`.

#### Scenario: Successful creation
- **WHEN** a request supplies a `playlist` value that is a bare YouTube playlist ID that exists on YouTube with the title "Lofi beats", a valid path, and a valid quality
- **THEN** the system persists a playlist record with that ID, the name "Lofi beats", that path and quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its name and kind

#### Scenario: Successful creation from a YouTube URL
- **WHEN** a request supplies a `playlist` value that is a full YouTube playlist URL (e.g. `https://www.youtube.com/playlist?list=PLabc123`) whose extracted playlist ID exists on YouTube, a valid path, and a valid quality
- **THEN** the system extracts the playlist ID from the URL, persists a playlist record with that ID, its YouTube title as the name, that path and quality, kind `youtube_linked`, and a creation timestamp, and returns the created playlist including its kind

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

## ADDED Requirements

### Requirement: Preview a YouTube Playlist
The system SHALL provide an HTTP endpoint that, given a `playlist` value that is either a YouTube playlist ID or a YouTube playlist URL, looks the playlist up on YouTube and returns its playlist ID, its YouTube title, and the number of videos YouTube reports for it, without persisting anything, publishing any event, or scheduling any task. It applies the same ID extraction and validation as Create Playlist, and the same fallback to the playlist ID when YouTube reports a blank title. The count is YouTube's own item count and may include videos that will not be downloaded, such as private ones.

The preview is independent of what is already tracked: previewing a playlist that is already stored SHALL return its YouTube data like any other.

#### Scenario: Previewing a playlist by ID
- **WHEN** a request supplies a bare playlist ID of a YouTube playlist titled "Lofi beats" with 42 videos
- **THEN** the system returns that playlist ID, the title "Lofi beats" and the count 42, and storage is unchanged

#### Scenario: Previewing a playlist by URL
- **WHEN** a request supplies a YouTube playlist URL (e.g. `https://www.youtube.com/watch?v=vid1&list=PLabc123`)
- **THEN** the system returns the extracted playlist ID `PLabc123` together with its YouTube title and video count

#### Scenario: Previewing a playlist with a blank title
- **WHEN** a request identifies a YouTube playlist whose title YouTube reports as empty
- **THEN** the system returns the playlist ID as the title

#### Scenario: Previewing an invalid value
- **WHEN** a request supplies an empty value, a URL that is not a recognized YouTube URL, or a YouTube URL without a `list` parameter
- **THEN** the system returns a bad request with a meaningful error description without contacting YouTube

#### Scenario: Previewing a nonexistent playlist
- **WHEN** a request supplies a playlist ID that does not correspond to an existing, accessible YouTube playlist
- **THEN** the system returns a not found response with a meaningful error description

#### Scenario: YouTube unavailable
- **WHEN** the YouTube lookup itself fails
- **THEN** the system returns a bad gateway response with a meaningful error description
