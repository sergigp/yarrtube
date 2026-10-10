## MODIFIED Requirements

### Requirement: Filesystem Reconciliation Against Recorded Downloads
The system SHALL, for every playlist regardless of kind, compare the files
present in that playlist's show directory (recursively through its season
folders) against the recorded filename and recorded thumbnail filename of
each of its Downloaded videos, and heal any divergence it finds. It SHALL
treat as protected, and never sweep: the show files (`tvshow.nfo`,
`poster.jpg`), season folders themselves, every recorded filename and
thumbnail filename of any video regardless of status, the NFO sharing a
recorded filename's base name, and every file sharing the episode base name
of a video whose episode number is recorded but whose download or thumbnail
fetch has not completed yet, because such files are only recorded once the
download or fetch completes. Legacy per-video folders and flat files
recorded on a video SHALL be protected the same way.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's show directory
- **THEN** the system resets that video to PENDING, clears its recorded filename, thumbnail filename, and quality, keeps its episode number, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the playlist's show directory or one of its season folders that is not a show file, not a recorded filename or recorded thumbnail filename or NFO of any video, not a downloader-managed in-progress temporary file, and does not share the episode base name of a video whose number is recorded
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

### Requirement: Missing Metadata Recovery
The system SHALL, for every playlist, regenerate a downloaded video's
metadata during a reconcile pass whenever that video is recorded as
successfully downloaded but has no metadata recorded as populated for it,
without re-downloading the video's file. When such a video is still stored
in the previous layout (a per-video folder or flat file) and its publish
timestamp is now known, the pass SHALL also assign its episode number and
move its file set into the TV-show layout, recording the new filenames, so
videos the `migrate-layout` subcommand skipped converge on their own.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the playlist that is recorded as downloaded but has no metadata recorded as populated
- **THEN** the system attempts to generate that video's metadata again, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a reconcile pass finds a video belonging to the playlist that is recorded as downloaded and already has metadata recorded as populated
- **THEN** the system does not attempt to regenerate its metadata

#### Scenario: Recovered video still in the previous layout
- **WHEN** a reconcile pass regenerates the metadata of a downloaded video whose recorded file is still in a per-video folder, and the metadata carries its publish timestamp
- **THEN** the pass assigns the video's episode number, moves its file, thumbnail and NFO under the episode base name in the season folder, removes the empty per-video folder, and records the new filenames

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
