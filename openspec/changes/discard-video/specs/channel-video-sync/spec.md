# Spec Delta

## MODIFIED Requirements

### Requirement: Channel Video Discovery Via yt-dlp
The system SHALL discover a channel's current videos by invoking `yt-dlp`
against that channel's videos listing, requesting only its `video_limit`
most recent uploads plus one more for each of the channel's stored videos
whose download is Unavailable or Discarded (skipped videos don't count
toward the limit), and parsing each discovered video's ID and title from
structured (JSON) output rather than delimited text, so that a video title
containing arbitrary characters (including a `|` or other delimiter-like
character) cannot corrupt discovery of that video or any other.

#### Scenario: Channel has more uploads than its video limit
- **WHEN** a reconcile pass discovers a channel's videos
- **THEN** the system requests and receives at most `video_limit` videos plus the number of the channel's skipped videos, ordered most recent first

#### Scenario: A discovered video's title contains a delimiter-like character
- **WHEN** a reconcile pass discovers a video whose title contains a `|` character (or other text that would corrupt a delimited-text parse)
- **THEN** the system correctly records that video's exact title and ID, and no other discovered video's data is corrupted by it

#### Scenario: yt-dlp fails to list a channel's videos
- **WHEN** a recurring or on-demand reconcile pass's attempt to discover a channel's current videos fails (e.g. the channel is private, terminated, or `yt-dlp` errors)
- **THEN** the system does not alter any of that channel's stored videos, logs the failure, and does not treat it as a fatal error for the reconcile pass

#### Scenario: Skipped videos widen the request
- **WHEN** a reconcile pass discovers the videos of a channel with `video_limit` 3 that has one Unavailable and one Discarded stored video
- **THEN** the system requests at most 5 videos

### Requirement: Aged-Out Video Eviction (Channels)
The system SHALL delete a stored video's record when a reconcile pass for a
channel finds that it is no longer among the channel's current most recent uploads up to and
including the `video_limit`-th one whose download is not Unavailable or
Discarded — whether because it was deleted/unlisted on YouTube or because
newer uploads have pushed it past that position — SHALL log
each such eviction, and SHALL publish a `VideoRemovedFromChannel` event
carrying that video's title, its recorded filename (if any), and whether it
had been downloaded.

#### Scenario: Previously stored video no longer among the channel's most recent uploads
- **WHEN** a reconcile pass finds that a video previously stored for a channel is no longer among that channel's current `video_limit` most recent uploads
- **THEN** the system deletes its stored record and logs the eviction

#### Scenario: Eviction publishes a VideoRemovedFromChannel event
- **WHEN** a reconcile pass evicts a stored video's record for a channel
- **THEN** the system publishes a `VideoRemovedFromChannel` event containing the channel's handle, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

#### Scenario: A skipped video doesn't take a slot
- **WHEN** a reconcile pass for a channel with `video_limit` 3 lists uploads A, B, C, D (most recent first) and B is stored as Unavailable
- **THEN** A, B, C and D are all kept, so the channel has 3 videos that aren't skipped

#### Scenario: Window ends at the limit-th unskipped video
- **WHEN** a reconcile pass for a channel with `video_limit` 2 lists uploads A, B, C (most recent first), none skipped, and C was stored
- **THEN** C is evicted
