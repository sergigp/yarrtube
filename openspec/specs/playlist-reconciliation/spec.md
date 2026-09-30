# playlist-reconciliation Specification

## Purpose

Keeps every tracked playlist's stored videos and downloaded files converged
onto their declared desired state — YouTube's membership for a YouTube-linked
playlist, and the database's own record of what should be on disk for every
playlist — through a recurring, self-healing reconcile pass rather than a
one-shot sync.

## Requirements

### Requirement: Reconcile On Playlist Creation
The system SHALL run one reconcile pass for a playlist as soon as practical
after it is created, regardless of the playlist's kind.

#### Scenario: Newly created YouTube-linked playlist gets reconciled
- **WHEN** a YouTube-linked playlist is successfully created
- **THEN** the system fetches its current members from the YouTube Data API and persists them

#### Scenario: Newly created custom playlist gets reconciled
- **WHEN** a custom playlist is successfully created
- **THEN** the system runs a reconcile pass that makes no YouTube API request and completes as a no-op, since the playlist has no videos yet

### Requirement: Recurring Reconciliation
The system SHALL schedule the next reconcile of a playlist after every
reconcile attempt that runs to completion, at a configurable interval,
regardless of the playlist's kind, whether any videos were added or removed,
or whether any file was healed.

#### Scenario: Reconcile finds no changes
- **WHEN** a playlist reconcile completes and finds no membership or filesystem changes
- **THEN** the system still schedules the next reconcile of that playlist after the configured interval

#### Scenario: Reconcile finds changes
- **WHEN** a playlist reconcile completes and finds membership and/or filesystem changes
- **THEN** the system schedules the next reconcile of that playlist after the configured interval, in addition to persisting the changes

### Requirement: Configurable Reconcile Interval
The system SHALL read the recurring reconcile interval from a configuration
value, defaulting to 3600 seconds when it is not set.

#### Scenario: Interval configured
- **WHEN** a reconcile interval configuration value is present
- **THEN** the system uses it as the delay before the next reconcile

#### Scenario: Interval not configured
- **WHEN** no reconcile interval configuration value is present
- **THEN** the system defaults to a 3600 second delay before the next reconcile

### Requirement: YouTube Membership Diff Applies Only To YouTube-Linked Playlists
The system SHALL fetch and diff a playlist's membership against YouTube only
when the playlist's kind is YouTube-linked. It SHALL NOT make any YouTube API
request for a custom playlist's reconcile pass.

#### Scenario: YouTube-linked playlist reconciled
- **WHEN** a reconcile pass runs for a YouTube-linked playlist
- **THEN** the system fetches its current members from the YouTube Data API and diffs them against stored videos

#### Scenario: Custom playlist reconciled
- **WHEN** a reconcile pass runs for a custom playlist
- **THEN** the system makes no YouTube API request and does not alter the playlist's video membership

### Requirement: Only Watchable Items Count As Playlist Members (YouTube-Linked Playlists)
The system SHALL treat an item of the source YouTube playlist as a member of
the playlist only when YouTube reports its privacy status as public or
unlisted. Items reported as private, items with no privacy status (such as
deleted videos), and items with any other privacy status SHALL be ignored by
the membership diff, as if they were not in the playlist at all.

#### Scenario: Playlist contains a private video not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a private item that has no stored record for that playlist
- **THEN** the system does not store it, does not fetch its thumbnail, and does not publish a `VideoAddedToPlaylist` event for it

#### Scenario: Playlist contains a deleted video
- **WHEN** a reconcile pass for a YouTube-linked playlist finds an item with no privacy status
- **THEN** the system ignores it the same way as a private item

#### Scenario: Playlist contains an unlisted video
- **WHEN** a reconcile pass for a YouTube-linked playlist finds an unlisted item that has no stored record for that playlist
- **THEN** the system stores it with status PENDING, the same as a public item

#### Scenario: Stored video that is not downloaded becomes private
- **WHEN** a reconcile pass for a YouTube-linked playlist finds that a stored video that is not downloaded is now private on YouTube
- **THEN** the system deletes its stored record and publishes a `VideoRemovedFromPlaylist` event, the same as for a video no longer in the playlist

#### Scenario: Downloaded video becomes private
- **WHEN** a reconcile pass for a YouTube-linked playlist finds that a downloaded video is now private on YouTube
- **THEN** the system deletes its stored record and publishes a `VideoRemovedFromPlaylist` event marking it as downloaded, so its local file is removed

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

### Requirement: Video Position Tracking (YouTube-Linked Playlists)
The system SHALL update a stored video's recorded position to match its
current position in the source YouTube playlist on every reconcile pass,
regardless of whether the video is newly discovered or already stored,
so that a playlist reordered on YouTube is reflected the next time it is
reconciled.

#### Scenario: Video's position changed on YouTube since the last reconcile
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a stored video whose position in the source YouTube playlist differs from its previously recorded position
- **THEN** the system updates the stored position to match

#### Scenario: Video's position unchanged since the last reconcile
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a stored video whose position in the source YouTube playlist matches its previously recorded position
- **THEN** the system leaves the stored position unchanged

### Requirement: Removed Video Cleanup (YouTube-Linked Playlists)
The system SHALL delete a stored video's record when a reconcile pass for a
YouTube-linked playlist finds that it is no longer present in the source
YouTube playlist, SHALL log each such deletion, and SHALL publish a
`VideoRemovedFromPlaylist` event carrying that video's title, its recorded
filename (if any), and whether it had been downloaded.

#### Scenario: Previously stored video no longer in the YouTube playlist
- **WHEN** a reconcile pass finds that a video previously stored for a YouTube-linked playlist is no longer a member of that playlist on YouTube
- **THEN** the system deletes its stored record and logs the deletion

#### Scenario: Deletion publishes a VideoDeleted event
- **WHEN** a reconcile pass deletes a stored video's record because it is no longer in the source YouTube playlist
- **THEN** the system publishes a `VideoRemovedFromPlaylist` event containing the playlist ID, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

### Requirement: Reconcile Skipped For a Deleted Playlist
The system SHALL NOT contact YouTube, persist any videos, sweep the
filesystem, or schedule a further reconcile when a reconcile pass runs for a
playlist that no longer exists.

#### Scenario: Scheduled reconcile runs after its playlist was deleted
- **WHEN** a reconcile pass is attempted for a playlist ID that no longer exists in storage
- **THEN** the system makes no YouTube request, persists nothing, performs no filesystem sweep, and does not schedule another reconcile for that playlist ID

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

### Requirement: Permanently Failed Video Recovery
The system SHALL, for every playlist regardless of kind, during each reconcile pass reset any video that has been permanently errored for at least 24 hours back to PENDING and trigger a fresh download of it. A video that became permanently errored less than 24 hours before the pass SHALL be left permanently errored and SHALL NOT have a download triggered by that pass. There is no limit on how many times a given video may be recovered this way.

#### Scenario: Permanently errored video found during reconcile
- **WHEN** a reconcile pass finds a video that has been permanently errored for 24 hours or more
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Video permanently errored less than 24 hours ago
- **WHEN** a reconcile pass finds a video that became permanently errored less than 24 hours earlier
- **THEN** the system leaves the video permanently errored and does not trigger a download for it

#### Scenario: Video errors again after being recovered
- **WHEN** a video that was previously reset by reconcile-driven recovery fails and permanently errors again
- **THEN** it is recovered again by the first reconcile pass that runs at least 24 hours after it errored, the same as any other permanently errored video

### Requirement: On-Demand Reconciliation
The system SHALL provide an HTTP endpoint that runs one reconcile pass for a
playlist immediately, on request, using the same membership-diff and
filesystem-healing behavior as the recurring and creation-triggered passes
(YouTube membership diff for YouTube-linked playlists, filesystem healing
for every playlist), regardless of the playlist's kind, without scheduling
or otherwise affecting any recurring reconcile task.

#### Scenario: On-demand reconcile of a YouTube-linked playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `youtube_linked`
- **THEN** the system fetches its current members from the YouTube Data API,
  diffs them against stored videos, and reconciles the filesystem

#### Scenario: On-demand reconcile of a custom playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `custom`
- **THEN** the system makes no YouTube API request and reconciles the
  filesystem

#### Scenario: On-demand reconcile does not affect the recurring schedule
- **WHEN** an on-demand reconcile runs for a playlist, whether or not it
  already has a future reconcile task pending (e.g. one scheduled by
  creation or the last recurring pass)
- **THEN** the system does not create, cancel, or otherwise modify any
  reconcile task; whatever was pending before the on-demand reconcile ran
  remains pending, unchanged, afterward — repeating the on-demand reconcile
  any number of times never changes the number of pending reconcile tasks

#### Scenario: On-demand reconcile of a nonexistent playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  does not exist in storage
- **THEN** the system makes no YouTube request, persists nothing, and
  performs no filesystem sweep, and responds without error, the same as the
  recurring reconcile task does for a deleted playlist

### Requirement: Missing Metadata Recovery
The system SHALL, for every playlist, regenerate a downloaded video's
metadata during a reconcile pass whenever that video is recorded as
successfully downloaded but has no metadata recorded as populated for it,
without re-downloading the video's file.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the playlist that is recorded as downloaded but has no metadata recorded as populated
- **THEN** the system attempts to generate that video's metadata again, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the playlist that is recorded as downloaded and already has metadata recorded as populated
- **THEN** the system does not attempt to regenerate its metadata

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

### Requirement: Reconcile Preserves Concurrent Download Progress
A reconcile pass refreshing a stored video's title or position SHALL change
only those fields, never the video's download status or any field set by
its download, so a download that progresses while the pass runs is not
reverted.

#### Scenario: Download completes while a reconcile refreshes the video's title
- **WHEN** a video's download changes its status between the time a reconcile pass reads that video and the time the pass stores its refreshed title
- **THEN** the video keeps the status and download fields set by the download, and gains the refreshed title
