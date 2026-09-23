## MODIFIED Requirements

### Requirement: Create Channel
The system SHALL provide an HTTP endpoint that creates a channel given a `channel` value that is either a YouTube channel handle (e.g. `@somechannel`) or a YouTube channel URL carrying a handle (e.g. `https://www.youtube.com/@somechannel`), a quality tier (`high`, `mid`, or `low`), a video limit that is an integer from 1 to 1000 inclusive, the number of the channel's most recent videos to keep synced, and a storage path. The system SHALL extract the handle from the `channel` value when it is a URL, resolve that handle against the YouTube Data API to confirm it corresponds to an existing, accessible channel and to obtain that channel's immutable YouTube channel ID and display title, and store the channel with the handle as its ID, the resolved title as its name, the resolved immutable channel ID, the quality, the video limit, the path, and a system-generated creation timestamp.

The stored ID is the handle, not the immutable channel ID, so the HTTP surface stays human-readable (e.g. `DELETE /channels/@somechannel`). The immutable ID is stored alongside it because a handle can be changed later by the channel's owner, while the immutable ID cannot.

The storage path identifies where the channel's videos are saved, relative to the configured videos root directory, and may contain multiple `/`-separated segments to express nested subdirectories (e.g. `creators/somechannel`), the same as a playlist's storage path.

#### Scenario: Successful creation from a bare handle
- **WHEN** a request supplies a `channel` value that is a bare handle that resolves to an existing YouTube channel, a valid quality, a valid video limit, and a valid path
- **THEN** the system persists a channel record with that handle as its ID, the channel's resolved title as its name, the resolved immutable channel ID, the quality, the video limit, the path, and a creation timestamp, and returns the created channel

#### Scenario: Successful creation from a channel URL
- **WHEN** a request supplies a `channel` value that is a full YouTube channel URL (e.g. `https://www.youtube.com/@somechannel`) whose handle resolves to an existing YouTube channel, a valid quality, a valid video limit, and a valid path
- **THEN** the system extracts the handle from the URL, persists a channel record the same way as the bare-handle case, and returns the created channel

#### Scenario: Duplicate channel handle
- **WHEN** a request supplies a `channel` value (bare handle or URL) whose handle already exists in storage
- **THEN** the system accepts the request but makes no change because the endpoint is idempotent, and returns the existing channel record

#### Scenario: Duplicate channel handle with a different quality
- **WHEN** a request supplies a `channel` value whose handle already exists in storage, with a quality value different from the stored record's
- **THEN** the system makes no change, ignores the request's quality value, and returns the existing record with its original quality

#### Scenario: Duplicate channel handle with a different video limit
- **WHEN** a request supplies a `channel` value whose handle already exists in storage, with a video limit different from the stored record's
- **THEN** the system makes no change, ignores the request's video limit, and returns the existing record with its original video limit

#### Scenario: Duplicate channel handle with a different path
- **WHEN** a request supplies a `channel` value whose handle already exists in storage, with a path value different from the stored record's
- **THEN** the system makes no change, ignores the request's path value, and returns the existing record with its original path

#### Scenario: Missing or empty channel value
- **WHEN** a request omits the `channel` field, or supplies an empty or whitespace-only value
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Handle missing its leading @
- **WHEN** a request supplies a bare `channel` value that does not start with `@`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Unrecognized URL
- **WHEN** a request supplies a `channel` value that is a URL but not a recognized YouTube URL
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: YouTube URL missing a handle
- **WHEN** a request supplies a `channel` value that is a recognized YouTube URL with no handle segment (e.g. a legacy `/channel/UC...` URL)
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Nonexistent YouTube channel
- **WHEN** a request supplies a `channel` value whose extracted handle does not correspond to an existing, accessible YouTube channel
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid quality
- **WHEN** a request omits quality, or supplies a value that is not `high`, `mid`, or `low`
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid video limit
- **WHEN** a request omits the video limit, or supplies a value that is not an integer from 1 to 1000 inclusive
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description

#### Scenario: Video limit above the maximum
- **WHEN** a request supplies a video limit greater than 1000, including values too large to fit in 32 bits
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request whose error description states the accepted range of 1 to 1000 and the value received

#### Scenario: Video limit at the bounds
- **WHEN** a request supplies a video limit of exactly 1 or exactly 1000, with every other field valid
- **THEN** the system persists the channel with that exact video limit and returns it unchanged

#### Scenario: Missing or invalid path
- **WHEN** a request omits path, supplies an empty path, or supplies a path that is absolute, contains a `..` segment, or contains an empty segment (e.g. leading/trailing/doubled `/`)
- **THEN** the system rejects the request without persisting anything and without checking YouTube and returns a bad request with a meaningful error description
