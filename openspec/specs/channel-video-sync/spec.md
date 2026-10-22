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
- **WHEN** a recurring or on-demand reconcile pass's attempt to discover a channel's current videos fails (e.g. the channel is private, terminated, or `yt-dlp` errors)
- **THEN** the system does not alter any of that channel's stored videos, logs the failure, and does not treat it as a fatal error for the reconcile pass

### Requirement: Not Yet Watchable Videos Are Not Discovered (Channels)
The system SHALL leave out of a channel's discovered videos any entry that
`yt-dlp` reports as an upcoming premiere or live stream, a stream that is
currently live, or a stream that has ended but is not yet processed. A
left-out entry SHALL be treated as absent from the channel's current
uploads: it is not stored, no download or thumbnail fetch is queued for it,
and the recency positions of the remaining discovered videos SHALL be
contiguous from `0`, most recent first. Once `yt-dlp` no longer reports the
entry that way, the next reconcile SHALL discover it like any other new
upload.

#### Scenario: Channel listing includes an upcoming premiere not yet stored
- **WHEN** a reconcile pass discovers a channel's videos and one entry is an upcoming premiere with no stored record
- **THEN** the system does not store it, does not queue its download or thumbnail fetch, and does not publish a `VideoAddedToChannel` event for it

#### Scenario: Channel listing includes a video that is live now
- **WHEN** a reconcile pass discovers a channel's videos and one entry is currently live, or has ended but is not yet processed
- **THEN** the system ignores it the same way as an upcoming premiere

#### Scenario: Positions skip the ignored entry
- **WHEN** a reconcile pass discovers an upcoming premiere listed first, followed by two regular uploads
- **THEN** the two regular uploads get positions `0` and `1`

#### Scenario: Stored video is listed as upcoming
- **WHEN** a reconcile pass finds that a video already stored for the channel is now listed as upcoming
- **THEN** the system evicts it as for any video no longer among the channel's current uploads

#### Scenario: Premiere becomes watchable
- **WHEN** a reconcile pass runs after a previously ignored premiere has aired and is no longer reported as upcoming, live or post-live
- **THEN** the system stores it with status PENDING and publishes a `VideoAddedToChannel` event, as for any newly seen upload

### Requirement: Reconcile On Channel Creation
The system SHALL run one reconcile pass for a channel as soon as practical after it is created, and SHALL write the channel's show files (`tvshow.nfo` and, when it has a stored avatar, `poster.jpg`; see `tv-show-layout`) in that pass. If discovering the channel's current videos fails, that pass SHALL fail, so it is retried with the creation event's processing rather than leaving the new channel empty until a recurring pass.

#### Scenario: Newly created channel gets reconciled
- **WHEN** a channel is successfully created
- **THEN** the system writes its show files, discovers its current most-recent videos (up to `video_limit`) and persists them

#### Scenario: Discovery fails during the initial reconcile
- **WHEN** the initial reconcile pass of a newly created channel fails to discover the channel's current videos
- **THEN** the system stores no video, publishes no event, schedules no recurring reconcile of the channel, and fails the `ChannelCreated` event's processing so it is retried

### Requirement: Recurring Channel Reconciliation
The system SHALL schedule the next reconcile of a channel after every reconcile attempt that runs to completion, after the channel's own reconcile interval as stored when the attempt finishes, regardless of whether any videos were added or evicted. When the channel no longer exists when the attempt finishes, the system SHALL schedule no further reconcile of it. A channel SHALL have at most one pending recurring reconcile: when a reconcile of the channel is already pending when the attempt finishes, the system SHALL schedule no further one.

#### Scenario: Reconcile finds no changes
- **WHEN** a channel reconcile completes and finds no membership changes
- **THEN** the system still schedules the next reconcile of that channel after the channel's reconcile interval

#### Scenario: Reconcile finds changes
- **WHEN** a channel reconcile completes and finds videos added and/or evicted
- **THEN** the system schedules the next reconcile of that channel after the channel's reconcile interval, in addition to persisting the changes

#### Scenario: Channels reconcile on their own intervals
- **WHEN** a channel with a 30 minute interval and a channel with a 1440 minute interval each complete a reconcile at the same time
- **THEN** the next reconcile of the first is scheduled 30 minutes later and of the second 1440 minutes later

#### Scenario: Interval changed during a reconcile
- **WHEN** a channel's interval is changed from 180 to 60 minutes while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules the next one 60 minutes later

#### Scenario: Channel deleted during a reconcile
- **WHEN** a channel is deleted while a reconcile of it is running
- **THEN** that reconcile, when it completes, schedules no further reconcile of the channel

#### Scenario: Reconcile already pending
- **WHEN** a reconcile of a channel completes while another reconcile of the same channel is already pending
- **THEN** the system schedules no further reconcile, and the pending one keeps its time

#### Scenario: Parallel reconcile chains converge
- **WHEN** a channel has several pending reconciles, left by a version without this rule
- **THEN** each extra one, when it completes while another is pending, schedules no successor, until the channel has a single pending reconcile

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
channel's show directory (recursively through its season folders) against
the recorded filename and recorded thumbnail filename of each of its
Downloaded videos, and heal any divergence it finds, the same way
`playlist-reconciliation`'s filesystem reconciliation does for playlists,
with the same protected set: show files, season folders, every recorded filename, thumbnail filename and subtitle filename of any video regardless of status, the NFO
sharing a recorded filename's base name, every file sharing the episode base
name of a video whose number is recorded but whose download or thumbnail
fetch has not completed yet, and legacy per-video folders or flat files
recorded on a video. Every file whose name starts with the base name of a Downloaded video's recorded filename followed by a dot SHALL also be protected while that video's subtitle state is Missing and it is not in the previous per-video-folder layout, because a subtitle file is placed before it is recorded.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its channel's show directory
- **THEN** the system resets that video to PENDING, resets its subtitle state to Missing and clears its recorded subtitle filename, clears its recorded filename and quality, keeps its episode number, and triggers a fresh download of it

#### Scenario: Downloaded video's file is present but not mp4
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's show directory but is not an mp4 container
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the channel's show directory or one of its season folders that is not a show file, not a recorded filename, recorded thumbnail filename, recorded subtitle filename or NFO of any video, not a downloader-managed in-progress temporary file, and does not share the episode base name of a video whose number is recorded
- **THEN** the system deletes that file

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's show directory and is an mp4 container
- **THEN** the system takes no action for that video

#### Scenario: A pending video's pre-fetched thumbnail folder is present
- **WHEN** a reconcile pass finds a file that matches a Pending or In Progress video's recorded thumbnail filename
- **THEN** the system does not treat that file as orphaned and does not delete it

#### Scenario: A download without a pre-fetched thumbnail is in progress
- **WHEN** a reconcile pass runs while a video of the channel is In Progress with a recorded episode number and no recorded thumbnail, and files sharing its episode base name are present in its season folder
- **THEN** the system does not treat those files as orphaned and does not delete them

#### Scenario: A thumbnail fetch without a recorded folder is in progress
- **WHEN** a reconcile pass runs while a Pending video of the channel has no recorded thumbnail filename and its thumbnail fetch is still staging outside the show directory
- **THEN** the pass finds nothing of it in the show directory and deletes nothing for it

#### Scenario: Show files are never swept
- **WHEN** a reconcile pass finds `tvshow.nfo`, `poster.jpg` or an empty season folder in the channel's show directory
- **THEN** the system leaves them in place

#### Scenario: Video renamed while its download is in progress
- **WHEN** a reconcile pass changes the title of an In Progress video of the channel
- **THEN** the system records the new title, and the files the download is writing under the base name derived from the previous title stay protected because the episode number is recorded

#### Scenario: Video's subtitle file is present
- **WHEN** a reconcile pass finds a file matching a video's recorded subtitle filename in its season folder
- **THEN** the system does not treat it as orphaned and does not delete it

#### Scenario: A subtitle file placed but not yet recorded
- **WHEN** a reconcile pass finds `<base name>.en.srt`, where `<base name>` is a Downloaded video's recorded filename without its extension, and that video's subtitle state is Missing
- **THEN** the system does not treat that file as orphaned and does not delete it, even if the video's title has changed since its download

### Requirement: Permanently Failed Video Recovery (Channels)
The system SHALL, for every channel, during each reconcile pass reset any
video that has been permanently errored for at least 24 hours back to
PENDING and trigger a fresh download of it. A video that became
permanently errored less than 24 hours before the pass SHALL be left
permanently errored and SHALL NOT have a download triggered by that pass.
There is no limit on how many times a given video may be recovered this way.
An excluded video SHALL NOT be recovered by this pass regardless of how long
ago it was excluded.

#### Scenario: Permanently errored video found during reconcile
- **WHEN** a reconcile pass finds a video that has been permanently errored for 24 hours or more
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Video permanently errored less than 24 hours ago
- **WHEN** a reconcile pass finds a video that became permanently errored less than 24 hours earlier
- **THEN** the system leaves the video permanently errored and does not trigger a download for it

#### Scenario: Excluded video is never recovered
- **WHEN** a reconcile pass finds an excluded video, however long ago it was excluded
- **THEN** the system leaves the video excluded and does not trigger a download for it

### Requirement: Missing Metadata Recovery (Channels)
The system SHALL, for every channel, regenerate a downloaded video's
metadata during a reconcile pass whenever that video is recorded as
successfully downloaded but has no metadata recorded as populated for it,
without re-downloading the video's file. When such a video is still stored
in the previous layout and its publish timestamp is now known, the pass
SHALL also assign its episode number and move its file set into the TV-show
layout, recording the new filenames, the same way
`playlist-reconciliation` does.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the channel that is recorded as downloaded but has no metadata recorded as populated
- **THEN** the system attempts to generate that video's metadata again, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the channel that is recorded as downloaded and already has metadata recorded as populated
- **THEN** the system does not attempt to regenerate its metadata

#### Scenario: Recovered video still in the previous layout
- **WHEN** a reconcile pass regenerates the metadata of a downloaded video whose recorded file is still in a per-video folder, and the metadata carries its publish timestamp
- **THEN** the pass assigns the video's episode number, moves its file, thumbnail and NFO under the episode base name in the season folder, removes the empty per-video folder, and records the new filenames

### Requirement: Missing Thumbnail Recovery (Channels)
The system SHALL, for every channel, schedule a thumbnail fetch task during
a reconcile pass for each video that has no recorded thumbnail filename,
regardless of its download status except as listed below, without affecting that video's download
status or triggering a redownload. The pass SHALL NOT fetch the thumbnail
itself, and SHALL NOT schedule one for:
- a video that already has a thumbnail fetch pending or running,
- a video the same pass resets for redownload,
- a video whose download is In Progress,
- a video whose download status is Excluded (permanently unavailable, so a fetch can never succeed) or Errored (recovery later resets it for redownload, and that download writes its own thumbnail),
- a video the same pass newly added: its thumbnail fetch is scheduled in reaction to its `VideoAddedToChannel` event (see `video-thumbnails`), so recovery only covers videos already stored before the pass.

#### Scenario: Video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename and no thumbnail fetch pending or running for it
- **THEN** the system schedules a thumbnail fetch task for that video

#### Scenario: Video already has a recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel that already has a recorded thumbnail filename
- **THEN** the system does not schedule a thumbnail fetch for it

#### Scenario: Video newly added by the same pass
- **WHEN** a reconcile pass adds a new video to the channel, and that video has no recorded thumbnail filename
- **THEN** the pass itself does not schedule a thumbnail fetch for that video; its fetch is scheduled in reaction to its `VideoAddedToChannel` event

#### Scenario: Thumbnail fetch already queued
- **WHEN** a reconcile pass finds a video with no recorded thumbnail filename that already has a thumbnail fetch task pending or running
- **THEN** the system does not schedule another thumbnail fetch for it

#### Scenario: Retried thumbnail fetch fails again
- **WHEN** a scheduled thumbnail fetch for a video fails
- **THEN** the video's status and any other recorded field are left unchanged, and the next reconcile pass schedules another fetch, unless the video's download status has meanwhile become Excluded or Errored

#### Scenario: Excluded video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename whose download status is Excluded
- **THEN** the system does not schedule a thumbnail fetch for it, on this or any later pass while it stays Excluded

#### Scenario: Errored video has no recorded thumbnail
- **WHEN** a reconcile pass finds a video belonging to the channel with no recorded thumbnail filename whose download status is Errored and that the same pass does not reset for redownload
- **THEN** the system does not schedule a thumbnail fetch for it

### Requirement: Reconcile Preserves Concurrent Download Progress (Channels)
A channel reconcile pass refreshing a stored video's title or position
SHALL change only those fields, never the video's download status or any
field set by its download, so a download that progresses while the pass
runs is not reverted.

#### Scenario: Download completes while a reconcile refreshes the video's title
- **WHEN** a video's download changes its status between the time a channel reconcile pass reads that video and the time the pass stores its refreshed title
- **THEN** the video keeps the status and download fields set by the download, and gains the refreshed title

### Requirement: Stranded Video Recovery (Channels)
The system SHALL, for every channel, during each reconcile pass trigger a download for every video of the channel that is stranded. A video is stranded when its download status is Pending, In Progress or Errored Retrying and it has no download task pending or running. The pass SHALL leave the video's download status unchanged, SHALL apply no cooldown, and SHALL NOT treat as stranded a video that the same pass newly adds or resets for redownload. A video whose status is Downloaded, Excluded or Errored is never stranded.

#### Scenario: Errored-retrying video with no download task
- **WHEN** a reconcile pass finds a video in status Errored Retrying with no download task pending or running for it
- **THEN** the system triggers a download for that video and leaves its status Errored Retrying

#### Scenario: In-progress video with no download task
- **WHEN** a reconcile pass finds a video in status In Progress with no download task pending or running for it
- **THEN** the system triggers a download for that video

#### Scenario: Non-terminal video whose download task is still queued
- **WHEN** a reconcile pass finds a video in status Pending, In Progress or Errored Retrying that has a download task pending or running
- **THEN** the system does not trigger another download for it

#### Scenario: Settled video is never stranded
- **WHEN** a reconcile pass finds a video in status Downloaded, Excluded or Errored with no download task
- **THEN** the stranded-video recovery does not trigger a download for it

### Requirement: Missing Subtitles Recovery (Channels)
The system SHALL, for every channel, schedule a subtitle fetch task during a reconcile pass for each Downloaded video whose subtitle state is Missing (see `video-subtitles`), without affecting that video's download status. The pass SHALL NOT fetch the subtitles itself, and SHALL NOT schedule one for a video that already has a subtitle fetch pending or running, nor for a video the same pass resets for redownload or newly adds, nor for a video still stored in the previous per-video-folder layout.

#### Scenario: Downloaded video with missing subtitles
- **WHEN** a reconcile pass finds a Downloaded video of the channel whose subtitle state is Missing and no subtitle fetch pending or running for it
- **THEN** the system schedules a subtitle fetch task for that video

#### Scenario: Library upgraded
- **WHEN** the first reconcile pass after upgrading runs for a channel whose Downloaded videos were stored before subtitles existed
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
