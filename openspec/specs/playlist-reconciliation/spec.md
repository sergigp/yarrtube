# playlist-reconciliation Specification

## Purpose

Keeps every tracked playlist's stored videos and downloaded files converged
onto their declared desired state — YouTube's membership for a YouTube-linked
playlist, and the database's own record of what should be on disk for every
playlist — through a recurring, self-healing reconcile pass rather than a
one-shot sync.

## Requirements

### Requirement: Recurring Reconciliation
The system SHALL schedule the next reconcile of a playlist after every
reconcile attempt that runs to completion, after the playlist's own reconcile
interval as stored when the attempt finishes, regardless of the playlist's
kind, whether any videos were added or removed, or whether any file was
healed. When the playlist no longer exists when the attempt finishes, the
system SHALL schedule no further reconcile of it. A playlist SHALL have at
most one pending recurring reconcile: when a reconcile of the playlist is
already pending when the attempt finishes, the system SHALL schedule no
further one.

#### Scenario: Reconcile finds no changes
- **WHEN** a playlist reconcile completes and finds no membership or filesystem changes
- **THEN** the system still schedules the next reconcile of that playlist after the playlist's reconcile interval

#### Scenario: Reconcile finds changes
- **WHEN** a playlist reconcile completes and finds membership and/or filesystem changes
- **THEN** the system schedules the next reconcile of that playlist after the playlist's reconcile interval, in addition to persisting the changes

#### Scenario: Playlists reconcile on their own intervals
- **WHEN** a playlist with a 30 minute interval and a playlist with a 1440 minute interval each complete a reconcile at the same time
- **THEN** the next reconcile of the first is scheduled 30 minutes later and of the second 1440 minutes later

#### Scenario: Interval changed during a reconcile
- **WHEN** a playlist's interval is changed from 180 to 60 minutes while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules the next one 60 minutes later

#### Scenario: Playlist deleted during a reconcile
- **WHEN** a playlist is deleted while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules no further reconcile of the playlist

#### Scenario: Reconcile already pending
- **WHEN** a reconcile of a playlist completes while another reconcile of the same playlist is already pending
- **THEN** the system schedules no further reconcile, and the pending one keeps its time

#### Scenario: Parallel reconcile chains converge
- **WHEN** a playlist has several pending reconciles, left by a version without this rule
- **THEN** each extra one, when it completes while another is pending, schedules no successor, until the playlist has a single pending reconcile

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

### Requirement: Not Yet Watchable Videos Are Not Playlist Members (YouTube-Linked Playlists)
The system SHALL treat an item of the source YouTube playlist that YouTube
reports as an upcoming or currently live broadcast as if it were not in the
playlist at all, even when its privacy status is public or unlisted. Once
YouTube no longer reports it as upcoming or live, the next reconcile SHALL
treat it like any other member.

#### Scenario: Playlist contains an upcoming premiere not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a public item that YouTube reports as an upcoming broadcast, with no stored record for that playlist
- **THEN** the system does not store it, does not fetch its thumbnail, and does not publish a `VideoAddedToPlaylist` event for it

#### Scenario: Playlist contains a video that is live now
- **WHEN** a reconcile pass for a YouTube-linked playlist finds an item that YouTube reports as a live broadcast
- **THEN** the system ignores it the same way as an upcoming item

#### Scenario: Stored video is reported as upcoming
- **WHEN** a reconcile pass for a YouTube-linked playlist finds that a stored video is now reported as an upcoming broadcast
- **THEN** the system deletes its stored record and publishes a `VideoRemovedFromPlaylist` event, the same as for a video no longer in the playlist

#### Scenario: Premiere becomes watchable
- **WHEN** a reconcile pass runs after a previously ignored premiere is no longer reported as upcoming or live
- **THEN** the system stores it with status PENDING, the same as any new public item

#### Scenario: Live-status lookup fails
- **WHEN** looking up whether a page's items are upcoming or live fails
- **THEN** the playlist's listing fails, as for any other listing failure

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
present in that playlist's show directory (recursively through its season
folders) against the recorded filename and recorded thumbnail filename of
each of its Downloaded videos, and heal any divergence it finds. It SHALL
treat as protected, and never sweep: the show files (`tvshow.nfo`,
`poster.jpg`), season folders themselves, every recorded filename, thumbnail filename and subtitle filename of any video regardless of status, the NFO sharing a
recorded filename's base name, and every file sharing the episode base name
of a video whose episode number is recorded but whose download or thumbnail
fetch has not completed yet, because such files are only recorded once the
download or fetch completes. Legacy per-video folders and flat files
recorded on a video SHALL be protected the same way. Every file whose name starts with the base name of a Downloaded video's recorded filename followed by a dot SHALL also be protected while that video's subtitle state is Missing and it is not in the previous per-video-folder layout, because a subtitle file is placed before it is recorded.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's show directory
- **THEN** the system resets that video to PENDING, resets its subtitle state to Missing and clears its recorded subtitle filename, clears its recorded filename, thumbnail filename, and quality, keeps its episode number, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the playlist's show directory or one of its season folders that is not a show file, not a recorded filename, recorded thumbnail filename, recorded subtitle filename or NFO of any video, not a downloader-managed in-progress temporary file, and does not share the episode base name of a video whose number is recorded
- **THEN** the system deletes that file

#### Scenario: A downloader-managed in-progress temporary file is present
- **WHEN** a reconcile pass finds a file in a season folder that the downloader itself uses to track an in-progress download
- **THEN** the system does not treat that file as orphaned and does not delete it

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its playlist's show directory
- **THEN** the system takes no action for that video

#### Scenario: Downloaded video's thumbnail file is present
- **WHEN** a reconcile pass finds a Downloaded video's recorded thumbnail file and the NFO sharing its base name in its season folder
- **THEN** the system does not treat them as orphaned and does not delete them

#### Scenario: A pending video's pre-fetched thumbnail folder is present
- **WHEN** a reconcile pass finds a file that matches a Pending or In Progress video's recorded thumbnail filename
- **THEN** the system does not treat that file as orphaned and does not delete it

#### Scenario: A download without a pre-fetched thumbnail is in progress
- **WHEN** a reconcile pass runs while a video of the playlist is In Progress with a recorded episode number and no recorded thumbnail, and files sharing its episode base name are present in its season folder
- **THEN** the system does not treat those files as orphaned and does not delete them

#### Scenario: A thumbnail fetch without a recorded folder is in progress
- **WHEN** a reconcile pass runs while a Pending video of the playlist has no recorded thumbnail filename and its thumbnail fetch is still staging outside the show directory
- **THEN** the pass finds nothing of it in the show directory and deletes nothing for it

#### Scenario: Show files are never swept
- **WHEN** a reconcile pass finds `tvshow.nfo`, `poster.jpg` or an empty season folder in the playlist's show directory
- **THEN** the system leaves them in place

#### Scenario: Video renamed while its download is in progress
- **WHEN** a reconcile pass changes the title of an In Progress video of the playlist
- **THEN** the system records the new title, and the files the download is writing under the base name derived from the previous title stay protected because the episode number is recorded

#### Scenario: Video's subtitle file is present
- **WHEN** a reconcile pass finds a file matching a video's recorded subtitle filename in its season folder
- **THEN** the system does not treat it as orphaned and does not delete it

#### Scenario: A subtitle file placed but not yet recorded
- **WHEN** a reconcile pass finds `<base name>.en.srt`, where `<base name>` is a Downloaded video's recorded filename without its extension, and that video's subtitle state is Missing
- **THEN** the system does not treat that file as orphaned and does not delete it, even if the video's title has changed since its download

### Requirement: Permanently Failed Video Recovery
The system SHALL, for every playlist regardless of kind, during each reconcile pass reset any video that has been permanently errored for at least 24 hours back to PENDING and trigger a fresh download of it. A video that became permanently errored less than 24 hours before the pass SHALL be left permanently errored and SHALL NOT have a download triggered by that pass. There is no limit on how many times a given video may be recovered this way. An excluded video SHALL NOT be recovered by this pass regardless of how long ago it was excluded.

#### Scenario: Permanently errored video found during reconcile
- **WHEN** a reconcile pass finds a video that has been permanently errored for 24 hours or more
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Video permanently errored less than 24 hours ago
- **WHEN** a reconcile pass finds a video that became permanently errored less than 24 hours earlier
- **THEN** the system leaves the video permanently errored and does not trigger a download for it

#### Scenario: Video errors again after being recovered
- **WHEN** a video that was previously reset by reconcile-driven recovery fails and permanently errors again
- **THEN** it is recovered again by the first reconcile pass that runs at least 24 hours after it errored, the same as any other permanently errored video

#### Scenario: Excluded video is never recovered
- **WHEN** a reconcile pass finds an excluded video, however long ago it was excluded
- **THEN** the system leaves the video excluded and does not trigger a download for it

### Requirement: Missing Metadata Recovery
The system SHALL, for every playlist, regenerate a downloaded video's
metadata during a reconcile pass whenever that video is recorded as
successfully downloaded but has no metadata recorded as populated for it,
without re-downloading the video's file. When such a video is still stored
in the previous layout (a per-video folder or flat file), the pass SHALL
also move its file set into the TV-show layout under its recorded episode
number (a playlist video is numbered in playlist order when first stored,
see `tv-show-layout`), recording the new filenames, so
videos the `migrate-layout` subcommand skipped converge on their own.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the playlist that is recorded as downloaded but has no metadata recorded as populated
- **THEN** the system attempts to generate that video's metadata again, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the playlist that is recorded as downloaded and already has metadata recorded as populated
- **THEN** the system does not attempt to regenerate its metadata

#### Scenario: Recovered video still in the previous layout
- **WHEN** a reconcile pass regenerates the metadata of a downloaded video whose recorded file is still in a per-video folder
- **THEN** the pass moves its file, thumbnail and NFO under the episode base name in the season folder, removes the empty per-video folder, and records the new filenames

### Requirement: Missing Thumbnail Recovery
The system SHALL, for every playlist, schedule a thumbnail fetch task during
a reconcile pass for each video that has no recorded thumbnail filename,
regardless of its download status except as listed below, without affecting that video's download
status or triggering a redownload. The pass SHALL NOT fetch the thumbnail
itself, and SHALL NOT schedule one for:
- a video that already has a thumbnail fetch pending or running,
- a video the same pass resets for redownload (its download writes its own thumbnail),
- a video whose download is In Progress,
- a video whose download status is Excluded (permanently unavailable, so a fetch can never succeed) or Errored (recovery later resets it for redownload, and that download writes its own thumbnail),
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
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass schedules another fetch, unless the video's download status has meanwhile become Excluded or Errored

#### Scenario: Excluded video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the playlist with no recorded thumbnail filename whose download status is Excluded
- **THEN** the system does not schedule a thumbnail fetch for it, on this or any later pass while it stays Excluded

#### Scenario: Errored video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the playlist with no recorded thumbnail filename whose download status is Errored and that the same pass does not reset for redownload
- **THEN** the system does not schedule a thumbnail fetch for it

### Requirement: Reconcile Preserves Concurrent Download Progress
A reconcile pass refreshing a stored video's title or position SHALL change
only those fields, never the video's download status or any field set by
its download, so a download that progresses while the pass runs is not
reverted.

#### Scenario: Download completes while a reconcile refreshes the video's title
- **WHEN** a video's download changes its status between the time a reconcile pass reads that video and the time the pass stores its refreshed title
- **THEN** the video keeps the status and download fields set by the download, and gains the refreshed title

### Requirement: Stranded Video Recovery
The system SHALL, for every playlist regardless of kind, during each reconcile pass trigger a download for every video of the playlist that is stranded. A video is stranded when its download status is Pending, In Progress or Errored Retrying and it has no download task pending or running. The pass SHALL leave the video's download status unchanged, SHALL apply no cooldown, and SHALL NOT treat as stranded a video that the same pass newly adds or resets for redownload. A video whose status is Downloaded, Excluded or Errored is never stranded.

#### Scenario: Errored-retrying video with no download task
- **WHEN** a reconcile pass finds a video in status Errored Retrying with no download task pending or running for it
- **THEN** the system triggers a download for that video and leaves its status Errored Retrying

#### Scenario: Pending video with no download task
- **WHEN** a reconcile pass finds a video in status Pending with no download task pending or running for it
- **THEN** the system triggers a download for that video

#### Scenario: In-progress video with no download task
- **WHEN** a reconcile pass finds a video in status In Progress with no download task pending or running for it
- **THEN** the system triggers a download for that video

#### Scenario: Non-terminal video whose download task is still queued
- **WHEN** a reconcile pass finds a video in status Pending, In Progress or Errored Retrying that has a download task pending or running
- **THEN** the system does not trigger another download for it

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new Pending video to the playlist
- **THEN** the pass does not treat it as stranded; its download is triggered by its `VideoAddedToPlaylist` event

#### Scenario: Settled video is never stranded
- **WHEN** a reconcile pass finds a video in status Downloaded, Excluded or Errored with no download task
- **THEN** the stranded-video recovery does not trigger a download for it

### Requirement: Playlist Listing Failure Is Not Fatal
The system SHALL treat a failure to list a playlist's current items on
YouTube as non-fatal for a recurring or on-demand reconcile pass: it SHALL
leave every stored video of that playlist, and its membership, unchanged for
that pass, SHALL publish no membership event, and SHALL log the failure. The
rest of the pass SHALL still run from the stored videos, and a recurring pass
SHALL still schedule the next reconcile of the playlist. The initial
reconcile after creation is the exception (see "Initial Reconcile After
Playlist Creation").

#### Scenario: Listing fails during a recurring reconcile
- **WHEN** a recurring reconcile pass's attempt to list a playlist's current items fails (e.g. the playlist was deleted or made private on YouTube, or the YouTube Data API errors)
- **THEN** the system leaves that playlist's stored videos unchanged, publishes no `VideoAddedToPlaylist` or `VideoRemovedFromPlaylist` event, logs the failure, and does not treat the pass as failed

#### Scenario: Filesystem steps still run when listing fails
- **WHEN** a recurring or on-demand reconcile pass's attempt to list a playlist's current items fails and a stored video of that playlist needs healing (e.g. its downloaded file is missing)
- **THEN** the system still heals it the same way as in a pass whose listing succeeded

#### Scenario: Next reconcile still scheduled when listing fails
- **WHEN** a recurring reconcile pass's attempt to list a playlist's current items fails
- **THEN** the system still schedules the next reconcile of that playlist after the configured interval

#### Scenario: On-demand reconcile when listing fails
- **WHEN** an on-demand reconcile of an existing playlist runs and its attempt to list the playlist's current items fails
- **THEN** the system leaves that playlist's stored videos unchanged and responds without error

### Requirement: Initial Reconcile After Playlist Creation
The system SHALL run one reconcile pass for a playlist as soon as practical
after it is created, and SHALL write the playlist's show files (see
`tv-show-layout`) in that pass. If listing the playlist's current items
fails, that pass SHALL fail, so it is retried with the creation event's
processing rather than leaving the new playlist empty until a recurring pass.

#### Scenario: Newly created YouTube-linked playlist gets reconciled
- **WHEN** a YouTube-linked playlist is successfully created
- **THEN** the system writes its `tvshow.nfo`, fetches its current members from the YouTube Data API and persists them

#### Scenario: Listing fails during the initial reconcile
- **WHEN** the initial reconcile pass of a newly created playlist fails to list the playlist's current items
- **THEN** the system stores no video, publishes no event, schedules no recurring reconcile of the playlist, and fails the `PlaylistCreated` event's processing so it is retried

### Requirement: On-Demand Playlist Reconciliation
The system SHALL provide an HTTP endpoint that runs one reconcile pass for a
playlist immediately, on request, using the same membership-diff and
filesystem-healing behavior as the recurring and creation-triggered passes,
without scheduling or otherwise affecting any recurring reconcile task.

#### Scenario: On-demand reconcile of a YouTube-linked playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `youtube_linked`
- **THEN** the system fetches its current members from the YouTube Data API,
  diffs them against stored videos, and reconciles the filesystem

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

### Requirement: Missing Subtitles Recovery
The system SHALL, for every playlist, schedule a subtitle fetch task during a reconcile pass for each Downloaded video whose subtitle state is Missing (see `video-subtitles`), without affecting that video's download status. The pass SHALL NOT fetch the subtitles itself, and SHALL NOT schedule one for a video that already has a subtitle fetch pending or running, nor for a video the same pass resets for redownload or newly adds, nor for a video still stored in the previous per-video-folder layout.

#### Scenario: Downloaded video with missing subtitles
- **WHEN** a reconcile pass finds a Downloaded video of the playlist whose subtitle state is Missing and no subtitle fetch pending or running for it
- **THEN** the system schedules a subtitle fetch task for that video

#### Scenario: Library upgraded
- **WHEN** the first reconcile pass after upgrading runs for a playlist whose Downloaded videos were stored before subtitles existed
- **THEN** a subtitle fetch is scheduled for each of them

#### Scenario: Subtitles already fetched or unavailable
- **WHEN** a reconcile pass finds a Downloaded video whose subtitle state is Fetched or Unavailable
- **THEN** the system does not schedule a subtitle fetch for it

#### Scenario: Video not downloaded
- **WHEN** a reconcile pass finds a video with Missing subtitles whose download status is not Downloaded
- **THEN** the system does not schedule a subtitle fetch for it

#### Scenario: Subtitle fetch already queued
- **WHEN** a reconcile pass finds a Downloaded video with Missing subtitles that already has a subtitle fetch pending or running
- **THEN** the system does not schedule another subtitle fetch for it

#### Scenario: Video reset for redownload by the same pass
- **WHEN** a reconcile pass resets a Downloaded video with Missing subtitles for redownload because its file is missing
- **THEN** the pass does not schedule a subtitle fetch for it
