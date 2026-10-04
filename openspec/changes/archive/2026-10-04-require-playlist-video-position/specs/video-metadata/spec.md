## ADDED Requirements

### Requirement: Sorttitle From Playlist Position Or Publish Date
The system SHALL derive `sorttitle` as a fixed-width, zero-padded numeric
prefix followed by the video's title: for a video belonging to a playlist,
its position in that playlist; for a channel-tracked video, a prefix derived
from its YouTube publish date instead.

#### Scenario: Video belongs to a playlist
- **WHEN** a video's metadata is generated and it belongs to a playlist
- **THEN** its `sorttitle` is prefixed with its position in that playlist, zero-padded to a fixed width

#### Scenario: Video belongs to a tracked channel
- **WHEN** a video's metadata is generated and it belongs to a tracked channel
- **THEN** its `sorttitle` is prefixed with a zero-padded value derived from its YouTube publish date, not the channel's recency rank

## REMOVED Requirements

### Requirement: Sorttitle Reflects Playlist Order Or Publish Date
**Reason**: Restated without the custom-playlist / no-position case as "Sorttitle From Playlist Position Or Publish Date"; every playlist video now has a position.
**Migration**: None.
