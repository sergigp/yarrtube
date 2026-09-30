## MODIFIED Requirements

### Requirement: New Video Persistence (YouTube-Linked Playlists)
The system SHALL store each video found in a YouTube-linked playlist that is
not already stored for that playlist, with a status of PENDING and its
current position in the source YouTube playlist, and publish a
`VideoAddedToPlaylist` event for it. The reconcile pass SHALL NOT fetch the
video's thumbnail itself. The thumbnail is fetched by a separately scheduled
task in reaction to the video's addition (see `video-thumbnails`), so
persisting a new video and publishing its event SHALL NOT wait on any
thumbnail fetch.

#### Scenario: Playlist contains a video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video in the playlist that has no existing stored record for that playlist
- **THEN** the system stores it with status PENDING and its current playlist position, and publishes a `VideoAddedToPlaylist` event, without fetching its thumbnail during the pass

#### Scenario: Playlist contains a video already stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video that already has a stored record for that playlist
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAddedToPlaylist` event

#### Scenario: Thumbnail fetch fails for a newly discovered video
- **WHEN** a reconcile pass stores a newly discovered video and that video's later, separately scheduled thumbnail fetch fails
- **THEN** the video was still stored and its `VideoAddedToPlaylist` event still published, and every other newly discovered video in the same pass was persisted regardless

#### Scenario: Many new videos found in one pass
- **WHEN** a reconcile pass for a newly created YouTube-linked playlist finds many videos not yet stored
- **THEN** the pass persists all of them and publishes their `VideoAddedToPlaylist` events without spending time fetching any of their thumbnails, so their downloads can be scheduled promptly

### Requirement: Filesystem Reconciliation Against Recorded Downloads
The system SHALL, for every playlist regardless of kind, compare the files
present in that playlist's output directory against the recorded filename
and recorded thumbnail filename of each of its Downloaded videos, and heal
any divergence it finds. It SHALL also protect any video's (regardless of
status) recorded thumbnail folder from being swept as orphaned, since a
thumbnail may have been fetched for a video that has not yet been
downloaded. It SHALL also protect every folder that a download of an In
Progress or errored-but-retrying video of that playlist may be writing
into, even when that video has no recorded thumbnail, because such a
download's folder is not recorded until the download completes.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename, thumbnail filename, and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the playlist's output directory that is not the recorded filename or recorded thumbnail filename of any currently Downloaded video, is not a downloader-managed in-progress temporary file, and is not a folder protected for an In Progress or errored-but-retrying video
- **THEN** the system deletes that file

#### Scenario: A downloader-managed in-progress temporary file is present
- **WHEN** a reconcile pass finds a file in the playlist's output directory that the downloader itself uses to track an in-progress download
- **THEN** the system does not treat that file as orphaned and does not delete it

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its playlist's output directory
- **THEN** the system takes no action for that video

#### Scenario: Downloaded video's thumbnail file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded thumbnail file is present in its playlist's output directory
- **THEN** the system does not treat that file as orphaned and does not delete it

#### Scenario: A pending video's pre-fetched thumbnail folder is present
- **WHEN** a reconcile pass finds a folder on disk that matches a Pending or In Progress video's recorded thumbnail filename
- **THEN** the system does not treat that folder as orphaned and does not delete it

#### Scenario: A download without a pre-fetched thumbnail is in progress
- **WHEN** a reconcile pass runs while a video of the playlist is In Progress with no recorded thumbnail filename, and the folder its download is writing into is present on disk
- **THEN** the system does not treat that folder as orphaned and does not delete it

### Requirement: Missing Thumbnail Recovery
The system SHALL, for every playlist, schedule a thumbnail fetch task during
a reconcile pass for each video that has no recorded thumbnail filename,
regardless of its download status, without affecting that video's download
status or triggering a redownload. The pass SHALL NOT fetch the thumbnail
itself, and SHALL NOT schedule one for:
- a video that already has a thumbnail fetch pending or running,
- a video the same pass resets for redownload (its download writes its own thumbnail),
- a video whose download is In Progress,
- a video the same pass newly added: its thumbnail fetch is scheduled in reaction to its `VideoAddedToPlaylist` event (see `video-thumbnails`), so recovery only covers videos already stored before the pass.

#### Scenario: Video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the playlist with no recorded thumbnail filename and no thumbnail fetch pending or running for it
- **THEN** the system schedules a thumbnail fetch task for that video

#### Scenario: Video already has a recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the playlist that already has a recorded thumbnail filename
- **THEN** the system does not schedule a thumbnail fetch for it

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new video to the playlist, and that video has no recorded thumbnail filename
- **THEN** the pass itself does not schedule a thumbnail fetch for that video; its fetch is scheduled in reaction to its `VideoAddedToPlaylist` event

#### Scenario: Thumbnail fetch already queued
- **WHEN** a reconcile pass finds a video with no recorded thumbnail filename that already has a thumbnail fetch task pending or running
- **THEN** the system does not schedule another thumbnail fetch for it

#### Scenario: Retried thumbnail fetch fails again
- **WHEN** a scheduled thumbnail fetch for a video fails
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass schedules another fetch

## ADDED Requirements

### Requirement: Reconcile Preserves Concurrent Download Progress
A reconcile pass refreshing a stored video's title or position SHALL change
only those fields, never the video's download status or any field set by
its download, so a download that progresses while the pass runs is not
reverted.

#### Scenario: Download completes while a reconcile refreshes the video's title
- **WHEN** a video's download changes its status between the time a reconcile pass reads that video and the time the pass stores its refreshed title
- **THEN** the video keeps the status and download fields set by the download, and gains the refreshed title
