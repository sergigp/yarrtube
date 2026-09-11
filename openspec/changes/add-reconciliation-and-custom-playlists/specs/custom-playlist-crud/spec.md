## Purpose

Lets a caller create a playlist that has no YouTube playlist behind it, and
add or remove individual videos on it by YouTube URL or ID, so playlists can
be curated entirely within this system instead of only ever mirroring an
existing YouTube playlist.

## ADDED Requirements

### Requirement: Create Custom Playlist
The system SHALL provide an HTTP endpoint that creates a playlist with kind
`custom`, given a caller-supplied ID, a name, a storage path, and a quality
tier (`high`, `mid`, or `low`). The ID SHALL be a well-formed UUID and SHALL
NOT already identify an existing playlist of either kind. The system SHALL
NOT make any YouTube API request as part of creating a custom playlist.

The storage path and quality tier follow the same rules as a YouTube-linked
playlist's (see `playlist-crud`).

#### Scenario: Successful creation
- **WHEN** a request supplies a well-formed, unused UUID, a valid name, a valid path, and a valid quality
- **THEN** the system persists a playlist record with kind `custom`, that ID, name, path, quality, and a creation timestamp, and returns the created playlist without having contacted YouTube

#### Scenario: ID is not a well-formed UUID
- **WHEN** a request supplies an ID that is not a well-formed UUID
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: ID already identifies an existing playlist
- **WHEN** a request supplies a UUID that already identifies a playlist, of either kind
- **THEN** the system rejects the request without making any change and returns a bad request with a meaningful error description

#### Scenario: Invalid name
- **WHEN** a request supplies an empty name, or a name containing characters that are unsafe for a filesystem folder name
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid path
- **WHEN** a request omits path, supplies an empty path, or supplies a path that is absolute, contains a `..` segment, or contains an empty segment
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Missing or invalid quality
- **WHEN** a request omits quality, or supplies a value that is not `high`, `mid`, or `low`
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

### Requirement: Custom Playlist Creation Publishes a Domain Event
The system SHALL publish a `PlaylistCreated` domain event, containing the
playlist's ID, whenever a custom playlist is successfully created.

#### Scenario: New custom playlist created
- **WHEN** a create-custom-playlist request results in a new playlist being persisted
- **THEN** the system publishes a `PlaylistCreated` event containing that playlist's ID

### Requirement: Add Video To Custom Playlist
The system SHALL provide an HTTP endpoint that adds a video to a custom
playlist, given a YouTube video URL or a bare video ID. It SHALL confirm the
video exists and is accessible via the YouTube Data API and fetch its title
from there before persisting anything, then store it with status PENDING
and publish a `VideoAdded` event.

#### Scenario: Successful add
- **WHEN** a request identifies a custom playlist and supplies a video URL or ID that exists and is accessible on YouTube
- **THEN** the system stores the video with status PENDING and its YouTube title, and publishes a `VideoAdded` event containing the playlist ID and video ID

#### Scenario: Video already a member of the playlist
- **WHEN** a request identifies a video that is already stored for that playlist
- **THEN** the system leaves its existing record unchanged and does not publish another `VideoAdded` event

#### Scenario: Video does not exist or is not accessible on YouTube
- **WHEN** a request supplies a video URL or ID that does not correspond to an existing, accessible YouTube video
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Malformed video URL or ID
- **WHEN** a request supplies a value that cannot be parsed as a YouTube video URL or a bare video ID
- **THEN** the system rejects the request without contacting YouTube or persisting anything, and returns a bad request with a meaningful error description

#### Scenario: Target playlist does not exist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system rejects the request without persisting anything and returns a bad request with a meaningful error description

#### Scenario: Target playlist is YouTube-linked
- **WHEN** a request identifies a playlist whose kind is YouTube-linked rather than custom
- **THEN** the system rejects the request without persisting anything and returns a bad request explaining that video membership for a YouTube-linked playlist cannot be changed manually

### Requirement: Remove Video From Custom Playlist
The system SHALL provide an HTTP endpoint that removes a video from a
custom playlist, identified by its video ID. On success it SHALL delete the
video's stored record and publish a `VideoDeleted` event carrying its title,
its recorded filename (if any), and whether it had been downloaded.

#### Scenario: Successful removal
- **WHEN** a request identifies a custom playlist and a video ID that is currently stored for it
- **THEN** the system deletes that video's stored record and publishes a `VideoDeleted` event containing the playlist ID, the video ID, its title, its recorded filename (if any), and whether it had been downloaded

#### Scenario: Video is not a member of the playlist
- **WHEN** a request identifies a video ID that is not currently stored for that playlist
- **THEN** the system makes no change to storage and returns a bad request with a meaningful error description

#### Scenario: Target playlist does not exist
- **WHEN** a request identifies a playlist ID that does not exist in storage
- **THEN** the system rejects the request without making any change and returns a bad request with a meaningful error description

#### Scenario: Target playlist is YouTube-linked
- **WHEN** a request identifies a playlist whose kind is YouTube-linked rather than custom
- **THEN** the system rejects the request without making any change and returns a bad request explaining that video membership for a YouTube-linked playlist cannot be changed manually
