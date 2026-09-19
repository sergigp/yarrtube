## MODIFIED Requirements

### Requirement: Add Video To Custom Playlist
The system SHALL provide an HTTP endpoint that adds a video to a custom
playlist, given a YouTube video URL or a bare video ID. It SHALL confirm the
video exists and is accessible via the YouTube Data API and fetch its title
from there before persisting anything, then store it with status PENDING,
attempt a best-effort thumbnail fetch for it (see `video-thumbnails`), and
publish a `VideoAdded` event. The thumbnail fetch's outcome SHALL NOT delay
or prevent the video's persistence or the event's publication.

#### Scenario: Successful add
- **WHEN** a request identifies a custom playlist and supplies a video URL or ID that exists and is accessible on YouTube
- **THEN** the system stores the video with status PENDING and its YouTube title, attempts to fetch its thumbnail, and publishes a `VideoAdded` event containing the playlist ID and video ID

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

#### Scenario: Thumbnail fetch fails
- **WHEN** a request successfully adds a video but its thumbnail fetch fails
- **THEN** the system still stores the video and still publishes the `VideoAdded` event, responding to the request as if the fetch had succeeded
