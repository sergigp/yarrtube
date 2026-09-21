## MODIFIED Requirements

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

## ADDED Requirements

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
