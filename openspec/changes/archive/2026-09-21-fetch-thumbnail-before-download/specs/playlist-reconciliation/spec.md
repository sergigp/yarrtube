## MODIFIED Requirements

### Requirement: New Video Persistence (YouTube-Linked Playlists)
The system SHALL store each video found in a YouTube-linked playlist that is
not already stored for that playlist, with a status of PENDING and its
current position in the source YouTube playlist. Before publishing a
`VideoAddedToPlaylist` event for it, the system SHALL attempt a best-effort
thumbnail fetch for that video (see `video-thumbnails`); the outcome of that
fetch SHALL NOT delay or prevent the video's persistence or the event's
publication.

#### Scenario: Playlist contains a video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video in the playlist that has no existing stored record for that playlist
- **THEN** the system stores it with status PENDING and its current playlist position, attempts to fetch its thumbnail, and publishes a `VideoAddedToPlaylist` event

#### Scenario: Playlist contains a video already stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video that already has a stored record for that playlist
- **THEN** the system leaves that record's status unchanged and does not publish another `VideoAddedToPlaylist` event

#### Scenario: Thumbnail fetch fails for a newly discovered video
- **WHEN** a reconcile pass stores a newly discovered video and its thumbnail fetch fails
- **THEN** the system still stores the video, still publishes the `VideoAddedToPlaylist` event, and continues persisting any other newly discovered videos in the same pass

### Requirement: Filesystem Reconciliation Against Recorded Downloads
The system SHALL, for every playlist regardless of kind, compare the files
present in that playlist's output directory against the recorded filename
and recorded thumbnail filename of each of its Downloaded videos, and heal
any divergence it finds. It SHALL also protect any video's (regardless of
status) recorded thumbnail folder from being swept as orphaned, since a
thumbnail may have been fetched for a video that has not yet been
downloaded.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename, thumbnail filename, and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the playlist's output directory that is not the recorded filename or recorded thumbnail filename of any currently Downloaded video, and is not a downloader-managed in-progress temporary file
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

## ADDED Requirements

### Requirement: Missing Thumbnail Recovery
The system SHALL, for every playlist, attempt a video's thumbnail fetch
again during a reconcile pass whenever that video has no recorded
thumbnail filename, regardless of its download status, without affecting
that video's download status or triggering a redownload.

#### Scenario: Video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the playlist with no recorded thumbnail filename
- **THEN** the system attempts to fetch that video's thumbnail again

#### Scenario: Video already has a recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the playlist that already has a recorded thumbnail filename
- **THEN** the system does not attempt to fetch its thumbnail again

#### Scenario: Retried thumbnail fetch fails again
- **WHEN** a reconcile pass's retried thumbnail fetch for a video fails
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass will retry again
