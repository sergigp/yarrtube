## MODIFIED Requirements

### Requirement: New Video Persistence (Channels)
The system SHALL store each video discovered for a channel that is not
already stored for that channel, with a status of PENDING and its current
recency position, and publish a `VideoAddedToChannel` event for it. The
reconcile pass SHALL NOT fetch the video's thumbnail itself. The thumbnail
is fetched by a separately scheduled task in reaction to the video's
addition (see `video-thumbnails`), so persisting a new video and publishing
its event SHALL NOT wait on any thumbnail fetch.

#### Scenario: Channel's most recent uploads include a video not yet stored
- **WHEN** a reconcile pass for a channel finds a video among its current most-recent uploads that has no existing stored record for that channel
- **THEN** the system stores it with status PENDING and its current recency position, and publishes a `VideoAddedToChannel` event, without fetching its thumbnail during the pass

#### Scenario: Channel's most recent uploads include a video already stored
- **WHEN** a reconcile pass for a channel finds a video that already has a stored record for that channel
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAddedToChannel` event

#### Scenario: Thumbnail fetch fails for a newly discovered video
- **WHEN** a reconcile pass stores a newly discovered video and that video's later, separately scheduled thumbnail fetch fails
- **THEN** the video was still stored and its `VideoAddedToChannel` event still published, and every other newly discovered video in the same pass was persisted regardless

### Requirement: Filesystem Reconciliation Against Recorded Downloads (Channels)
The system SHALL, for every channel, compare the files present in that
channel's output directory against the recorded filename of each of its
Downloaded videos, and heal any divergence it finds, the same way
`playlist-reconciliation`'s filesystem reconciliation does for playlists.
It SHALL also protect any video's (regardless of status) recorded
thumbnail folder from being swept as orphaned, since a thumbnail may have
been fetched for a video that has not yet been downloaded. It SHALL also
protect every folder that a download of an In Progress or
errored-but-retrying video of that channel may be writing into, even when
that video has no recorded thumbnail, because such a download's folder is
not recorded until the download completes.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its channel's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Downloaded video's file is present but not mp4
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's output directory but is not an mp4 container
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the channel's output directory that is not the recorded filename of any currently Downloaded video, is not a downloader-managed in-progress temporary file, and is not a folder protected for an In Progress or errored-but-retrying video
- **THEN** the system deletes that file

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's output directory and is an mp4 container
- **THEN** the system takes no action for that video

#### Scenario: A pending video's pre-fetched thumbnail folder is present
- **WHEN** a reconcile pass finds a folder on disk that matches a Pending or In Progress video's recorded thumbnail filename
- **THEN** the system does not treat that folder as orphaned and does not delete it

#### Scenario: A download without a pre-fetched thumbnail is in progress
- **WHEN** a reconcile pass runs while a video of the channel is In Progress with no recorded thumbnail filename, and the folder its download is writing into is present on disk
- **THEN** the system does not treat that folder as orphaned and does not delete it

### Requirement: Missing Thumbnail Recovery (Channels)
The system SHALL, for every channel, schedule a thumbnail fetch task during
a reconcile pass for each video that has no recorded thumbnail filename,
regardless of its download status, without affecting that video's download
status or triggering a redownload. The pass SHALL NOT fetch the thumbnail
itself, and SHALL NOT schedule one for:
- a video that already has a thumbnail fetch pending or running,
- a video the same pass resets for redownload,
- a video whose download is In Progress.

#### Scenario: Video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename and no thumbnail fetch pending or running for it
- **THEN** the system schedules a thumbnail fetch task for that video

#### Scenario: Video already has a recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel that already has a recorded thumbnail filename
- **THEN** the system does not schedule a thumbnail fetch for it

#### Scenario: Thumbnail fetch already queued
- **WHEN** a reconcile pass finds a video with no recorded thumbnail filename that already has a thumbnail fetch task pending or running
- **THEN** the system does not schedule another thumbnail fetch for it

#### Scenario: Retried thumbnail fetch fails again
- **WHEN** a scheduled thumbnail fetch for a video fails
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass schedules another fetch

## ADDED Requirements

### Requirement: Reconcile Preserves Concurrent Download Progress (Channels)
A channel reconcile pass refreshing a stored video's title or position
SHALL change only those fields, never the video's download status or any
field set by its download, so a download that progresses while the pass
runs is not reverted.

#### Scenario: Download completes while a reconcile refreshes the video's title
- **WHEN** a video's download changes its status between the time a channel reconcile pass reads that video and the time the pass stores its refreshed title
- **THEN** the video keeps the status and download fields set by the download, and gains the refreshed title
