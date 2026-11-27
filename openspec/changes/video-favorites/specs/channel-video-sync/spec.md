# Spec Delta

## MODIFIED Requirements

### Requirement: Channel Video Discovery Via yt-dlp
The system SHALL discover a channel's current videos by invoking `yt-dlp`
against that channel's videos listing, requesting only its `video_limit`
most recent uploads plus one more for each of the channel's stored videos
whose download is Unavailable or that is a favorite (Unavailable videos and
favorites don't count toward the limit; Discarded ones, stored or
remembered, do — see `video-discard` and `video-favorites`), and
parsing each discovered video's ID and title from
structured (JSON) output rather than delimited text, so that a video title
containing arbitrary characters (including a `|` or other delimiter-like
character) cannot corrupt discovery of that video or any other.

#### Scenario: Channel has more uploads than its video limit
- **WHEN** a reconcile pass discovers a channel's videos
- **THEN** the system requests and receives at most `video_limit` videos plus the number of the channel's Unavailable and favorite videos, ordered most recent first

#### Scenario: A discovered video's title contains a delimiter-like character
- **WHEN** a reconcile pass discovers a video whose title contains a `|` character (or other text that would corrupt a delimited-text parse)
- **THEN** the system correctly records that video's exact title and ID, and no other discovered video's data is corrupted by it

#### Scenario: yt-dlp fails to list a channel's videos
- **WHEN** a recurring or on-demand reconcile pass's attempt to discover a channel's current videos fails (e.g. the channel is private, terminated, or `yt-dlp` errors)
- **THEN** the system does not alter any of that channel's stored videos, logs the failure, and does not treat it as a fatal error for the reconcile pass

#### Scenario: Skipped videos widen the request
- **WHEN** a reconcile pass discovers the videos of a channel with `video_limit` 3 that has one Unavailable and one Discarded stored video
- **THEN** the system requests at most 4 videos

#### Scenario: Favorites widen the request
- **WHEN** a reconcile pass discovers the videos of a channel with `video_limit` 3 that has two favorite stored videos, one of them kept
- **THEN** the system requests at most 5 videos

### Requirement: Aged-Out Video Eviction (Channels)
The system SHALL delete a stored video's record when a reconcile pass for a
channel finds that it is no longer among the channel's current most recent uploads up to and
including the `video_limit`-th one whose download is not Unavailable and
that is not a favorite —
whether because it was deleted/unlisted on YouTube or because newer uploads have
pushed it past that position — unless it is a favorite, which is kept
instead (see "Favorites Are Kept (Channels)"). It
SHALL remember an evicted Discarded video's
YouTube id for the channel before deleting its record, SHALL log
each such eviction, and SHALL publish a `VideoRemovedFromChannel` event
carrying that video's title, its recorded filename (if any), and whether it
had been downloaded. A Discarded upload, stored or remembered by the
channel, SHALL take its slot within the limit.

#### Scenario: Previously stored video no longer among the channel's most recent uploads
- **WHEN** a reconcile pass finds that a video previously stored for a channel, not a favorite, is no longer among that channel's current `video_limit` most recent uploads
- **THEN** the system deletes its stored record and logs the eviction

#### Scenario: Eviction publishes a VideoRemovedFromChannel event
- **WHEN** a reconcile pass evicts a stored video's record for a channel
- **THEN** the system publishes a `VideoRemovedFromChannel` event containing the channel's handle, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

#### Scenario: A skipped video doesn't take a slot
- **WHEN** a reconcile pass for a channel with `video_limit` 3 lists uploads A, B, C, D (most recent first) and B is stored as Unavailable
- **THEN** A, B, C and D are all kept, so the channel has 3 videos that aren't Unavailable

#### Scenario: A favorite doesn't take a slot
- **WHEN** a reconcile pass for a channel with `video_limit` 3 lists uploads A, B, C, D (most recent first) and B is a stored favorite
- **THEN** A, B, C and D are all stored, so the channel has 3 videos besides its favorite

#### Scenario: A discarded video takes its slot
- **WHEN** a reconcile pass for a channel with `video_limit` 3 lists uploads A, B, C, D (most recent first) and B is stored as Discarded
- **THEN** A, B and C are kept, D is not stored, and the channel lists 2 videos

#### Scenario: A remembered discarded upload takes its slot
- **WHEN** a reconcile pass for a channel with `video_limit` 2 lists uploads A, B, C (most recent first) and the channel remembers B as discarded
- **THEN** only A is stored

#### Scenario: Evicting a discarded video remembers it
- **WHEN** a reconcile pass for a channel evicts a stored video whose download is Discarded
- **THEN** the channel remembers its YouTube id, then its stored record is deleted

#### Scenario: Window ends at the limit-th unskipped video
- **WHEN** a reconcile pass for a channel with `video_limit` 2 lists uploads A, B, C (most recent first), none Unavailable or a favorite, and C was stored
- **THEN** C is evicted

## ADDED Requirements

### Requirement: Favorites Are Kept (Channels)
A reconcile pass SHALL keep a stored favorite that is no longer among the channel's current uploads (aged past the window or gone from YouTube) instead of evicting it: its record, files and pending work stay, no `VideoRemovedFromChannel` is published, and it is marked kept. A video favorited while the pass runs SHALL be kept too.

#### Scenario: Favorite ages out of the window
- **WHEN** a reconcile pass for a channel with `video_limit` 2 lists uploads A, B and the stored favorite C is no longer listed
- **THEN** C stays stored and downloaded, is marked kept, and no `VideoRemovedFromChannel` is published for it

#### Scenario: Favorite deleted on YouTube
- **WHEN** a stored favorite of a channel was deleted on YouTube and the next reconcile pass doesn't list it
- **THEN** it stays stored with its files and is marked kept

#### Scenario: Favorite not downloaded yet ages out
- **WHEN** a favorite whose download is Retrying ages out of its channel's window
- **THEN** it is kept and its pending download stays scheduled

#### Scenario: Favorited during the pass
- **WHEN** a stored video is favorited while the listing that ages it out runs
- **THEN** it is kept, not evicted

#### Scenario: Unfavorited kept video
- **WHEN** a kept video of a channel is unfavorited and the next reconcile pass doesn't list it
- **THEN** it is evicted like any video no longer among the channel's uploads, and its files are deleted

### Requirement: Kept Videos Sit After The Listed Ones (Channels)
A reconcile pass SHALL place the channel's kept videos after its listed ones, in the order they had before, so the channel's videos stay ordered with the listed uploads first. A kept video that the channel lists again SHALL stop being kept and take its listed position.

#### Scenario: Kept video placed last
- **WHEN** a reconcile pass for a channel lists A, B and keeps the favorites C and D, C having been more recent than D
- **THEN** the channel's videos are ordered A, B, C, D

#### Scenario: Kept video listed again
- **WHEN** a kept video of a channel is among its listed uploads again, e.g. after its `video_limit` was raised
- **THEN** it is no longer kept and takes its recency position
