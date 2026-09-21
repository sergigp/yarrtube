# channel-video-sync Specification

## Purpose

Keeps a tracked channel's stored videos converged on its most recent
`video_limit` uploads via a recurring, self-healing reconcile pass — the
channel equivalent of `playlist-reconciliation` — so newly-uploaded videos
are downloaded automatically and videos that age out of that window are
cleaned up automatically.

## Requirements

### Requirement: Channel Video Discovery Via yt-dlp
The system SHALL discover a channel's current videos by invoking `yt-dlp`
against that channel's videos listing, requesting only its `video_limit`
most recent uploads, and parsing each discovered video's ID and title from
structured (JSON) output rather than delimited text, so that a video title
containing arbitrary characters (including a `|` or other delimiter-like
character) cannot corrupt discovery of that video or any other.

#### Scenario: Channel has more uploads than its video limit
- **WHEN** a reconcile pass discovers a channel's videos
- **THEN** the system requests and receives at most `video_limit` videos, ordered most recent first

#### Scenario: A discovered video's title contains a delimiter-like character
- **WHEN** a reconcile pass discovers a video whose title contains a `|` character (or other text that would corrupt a delimited-text parse)
- **THEN** the system correctly records that video's exact title and ID, and no other discovered video's data is corrupted by it

#### Scenario: yt-dlp fails to list a channel's videos
- **WHEN** a reconcile pass's attempt to discover a channel's current videos fails (e.g. the channel is private, terminated, or `yt-dlp` errors)
- **THEN** the system does not alter any of that channel's stored videos, logs the failure, and does not treat it as a fatal error for the reconcile pass

### Requirement: Reconcile On Channel Creation
The system SHALL run one reconcile pass for a channel as soon as practical after it is created.

#### Scenario: Newly created channel gets reconciled
- **WHEN** a channel is successfully created
- **THEN** the system discovers its current most-recent videos (up to `video_limit`) and persists them

### Requirement: Recurring Channel Reconciliation
The system SHALL schedule the next reconcile of a channel after every reconcile attempt that runs to completion, using the same configurable reconcile interval used for playlist reconciliation, regardless of whether any videos were added or evicted.

#### Scenario: Reconcile finds no changes
- **WHEN** a channel reconcile completes and finds no membership changes
- **THEN** the system still schedules the next reconcile of that channel after the configured interval

#### Scenario: Reconcile finds changes
- **WHEN** a channel reconcile completes and finds videos added and/or evicted
- **THEN** the system schedules the next reconcile of that channel after the configured interval, in addition to persisting the changes

### Requirement: New Video Persistence (Channels)
The system SHALL store each video discovered for a channel that is not
already stored for that channel, with a status of PENDING and its current
recency position. Before publishing a `VideoAddedToChannel` event for it,
the system SHALL attempt a best-effort thumbnail fetch for that video (see
`video-thumbnails`); the outcome of that fetch SHALL NOT delay or prevent
the video's persistence or the event's publication.

#### Scenario: Channel's most recent uploads include a video not yet stored
- **WHEN** a reconcile pass for a channel finds a video among its current most-recent uploads that has no existing stored record for that channel
- **THEN** the system stores it with status PENDING and its current recency position, attempts to fetch its thumbnail, and publishes a `VideoAddedToChannel` event

#### Scenario: Channel's most recent uploads include a video already stored
- **WHEN** a reconcile pass for a channel finds a video that already has a stored record for that channel
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAddedToChannel` event

#### Scenario: Thumbnail fetch fails for a newly discovered video
- **WHEN** a reconcile pass stores a newly discovered video for a channel and its thumbnail fetch fails
- **THEN** the system still stores the video, still publishes the `VideoAddedToChannel` event, and continues persisting any other newly discovered videos in the same pass

### Requirement: Video Position Tracking (Channels)
The system SHALL update a stored video's recorded position to match its
current recency rank among the channel's most recent uploads on every
reconcile pass, so newer uploads are reflected as such the next time the
channel is reconciled.

#### Scenario: A video's recency rank changed since the last reconcile
- **WHEN** a reconcile pass for a channel finds a stored video whose recency rank differs from its previously recorded position
- **THEN** the system updates the stored position to match

#### Scenario: A video's recency rank is unchanged since the last reconcile
- **WHEN** a reconcile pass for a channel finds a stored video whose recency rank matches its previously recorded position
- **THEN** the system leaves the stored position unchanged

### Requirement: Aged-Out Video Eviction (Channels)
The system SHALL delete a stored video's record when a reconcile pass for a
channel finds that it is no longer among the channel's current `video_limit`
most recent uploads — whether because it was deleted/unlisted on YouTube or
because newer uploads have pushed it past position `video_limit` — SHALL log
each such eviction, and SHALL publish a `VideoRemovedFromChannel` event
carrying that video's title, its recorded filename (if any), and whether it
had been downloaded.

#### Scenario: Previously stored video no longer among the channel's most recent uploads
- **WHEN** a reconcile pass finds that a video previously stored for a channel is no longer among that channel's current `video_limit` most recent uploads
- **THEN** the system deletes its stored record and logs the eviction

#### Scenario: Eviction publishes a VideoRemovedFromChannel event
- **WHEN** a reconcile pass evicts a stored video's record for a channel
- **THEN** the system publishes a `VideoRemovedFromChannel` event containing the channel's handle, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

### Requirement: Reconcile Skipped For a Deleted Channel
The system SHALL NOT invoke `yt-dlp`, persist any videos, or schedule a
further reconcile when a reconcile pass runs for a channel that no longer
exists.

#### Scenario: Scheduled reconcile runs after its channel was deleted
- **WHEN** a reconcile pass is attempted for a channel handle that no longer exists in storage
- **THEN** the system makes no `yt-dlp` invocation, persists nothing, and does not schedule another reconcile for that channel

### Requirement: On-Demand Channel Reconciliation
The system SHALL provide an HTTP endpoint that runs one reconcile pass for a
channel immediately, on request, using the same discovery, persistence, and
eviction behavior as the recurring and creation-triggered passes, without
scheduling or otherwise affecting any recurring reconcile task.

#### Scenario: On-demand reconcile of an existing channel
- **WHEN** a client requests an on-demand reconcile for a channel handle that exists in storage
- **THEN** the system discovers its current most-recent videos (up to `video_limit`), persists additions, and evicts aged-out videos

#### Scenario: On-demand reconcile does not affect the recurring schedule
- **WHEN** an on-demand reconcile runs for a channel, whether or not it already has a future reconcile task pending
- **THEN** the system does not create, cancel, or otherwise modify any reconcile task for that channel; repeating the on-demand reconcile any number of times never changes the number of pending reconcile tasks

#### Scenario: On-demand reconcile of a nonexistent channel
- **WHEN** a client requests an on-demand reconcile for a channel handle that does not exist in storage
- **THEN** the system makes no `yt-dlp` invocation, persists nothing, and responds without error, the same as the recurring reconcile task does for a deleted channel

### Requirement: Filesystem Reconciliation Against Recorded Downloads (Channels)
The system SHALL, for every channel, compare the files present in that
channel's output directory against the recorded filename of each of its
Downloaded videos, and heal any divergence it finds, the same way
`playlist-reconciliation`'s filesystem reconciliation does for playlists.
It SHALL also protect any video's (regardless of status) recorded
thumbnail folder from being swept as orphaned, since a thumbnail may have
been fetched for a video that has not yet been downloaded.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its channel's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Downloaded video's file is present but not mp4
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's output directory but is not an mp4 container
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the channel's output directory that is not the recorded filename of any currently Downloaded video, and is not a downloader-managed in-progress temporary file
- **THEN** the system deletes that file

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's output directory and is an mp4 container
- **THEN** the system takes no action for that video

#### Scenario: A pending video's pre-fetched thumbnail folder is present
- **WHEN** a reconcile pass finds a folder on disk that matches a Pending or In Progress video's recorded thumbnail filename
- **THEN** the system does not treat that folder as orphaned and does not delete it

### Requirement: Permanently Failed Video Recovery (Channels)
The system SHALL, for every channel, also reset any video whose status is
permanently errored back to PENDING and trigger a fresh download of it
during each reconcile pass, with no limit on how many times a given video
may be recovered this way.

#### Scenario: Permanently errored video found during reconcile
- **WHEN** a reconcile pass finds a video whose status is permanently errored
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

### Requirement: Missing Metadata Recovery (Channels)
The system SHALL, for every channel, regenerate a downloaded video's
metadata during a reconcile pass whenever that video is recorded as
successfully downloaded but has no metadata recorded as populated for it,
without re-downloading the video's file.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the channel that is recorded as downloaded but has no metadata recorded as populated
- **THEN** the system attempts to generate that video's metadata again, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the channel that is recorded as downloaded and already has metadata recorded as populated
- **THEN** the system does not attempt to regenerate its metadata

### Requirement: Missing Thumbnail Recovery (Channels)
The system SHALL, for every channel, attempt a video's thumbnail fetch
again during a reconcile pass whenever that video has no recorded
thumbnail filename, regardless of its download status, without affecting
that video's download status or triggering a redownload.

#### Scenario: Video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename
- **THEN** the system attempts to fetch that video's thumbnail again

#### Scenario: Video already has a recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel that already has a recorded thumbnail filename
- **THEN** the system does not attempt to fetch its thumbnail again

#### Scenario: Retried thumbnail fetch fails again
- **WHEN** a reconcile pass's retried thumbnail fetch for a video fails
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass will retry again
