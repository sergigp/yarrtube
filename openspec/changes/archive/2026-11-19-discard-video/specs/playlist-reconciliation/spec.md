# Spec Delta

## MODIFIED Requirements

### Requirement: New Video Persistence (YouTube-Linked Playlists)
The system SHALL store each video found in a YouTube-linked playlist that is
not already stored for that playlist and not remembered as discarded by it
(see `video-discard`), with a status of PENDING and its
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

#### Scenario: Playlist contains a remembered discarded video
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a video the playlist remembers as discarded
- **THEN** the system doesn't store it and publishes no `VideoAddedToPlaylist` event for it

### Requirement: Removed Video Cleanup (YouTube-Linked Playlists)
The system SHALL delete a stored video's record when a reconcile pass for a
YouTube-linked playlist finds that it is no longer present in the source
YouTube playlist, SHALL remember a removed Discarded video's YouTube id for
the playlist before deleting its record, SHALL log each such deletion, and
SHALL publish a
`VideoRemovedFromPlaylist` event carrying that video's title, its recorded
filename (if any), and whether it had been downloaded.

#### Scenario: Previously stored video no longer in the YouTube playlist
- **WHEN** a reconcile pass finds that a video previously stored for a YouTube-linked playlist is no longer a member of that playlist on YouTube
- **THEN** the system deletes its stored record and logs the deletion

#### Scenario: Deletion publishes a VideoDeleted event
- **WHEN** a reconcile pass deletes a stored video's record because it is no longer in the source YouTube playlist
- **THEN** the system publishes a `VideoRemovedFromPlaylist` event containing the playlist ID, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

#### Scenario: Removing a discarded video remembers it
- **WHEN** a reconcile pass deletes the record of a video whose download is Discarded because it is no longer in the source YouTube playlist
- **THEN** the playlist remembers its YouTube id, then its stored record is deleted


### Requirement: Filesystem Reconciliation Against Recorded Downloads
The system SHALL, for every playlist regardless of kind, compare the files
present in that playlist's show directory (recursively through its season
folders) against the recorded filename and recorded thumbnail filename of
each of its Downloaded videos, and heal any divergence it finds: a Downloaded
video whose recorded file is missing is reset by its video sync (see
`video-sync`), and every file left unaccounted for once every video of the
pass has been synced is deleted. It SHALL
treat as protected, and never sweep: the show files (`tvshow.nfo`,
`poster.jpg`), season folders themselves, every recorded filename, thumbnail filename and subtitle filename of any video regardless of status, the NFO sharing a
recorded filename's base name, and every media (`.mp4`), thumbnail (`.jpg`) or NFO (`.nfo`) file under the
episode base name of a video whose episode number is recorded, that is not Discarded, but whose download or thumbnail
fetch has not completed yet, because such files are only recorded once the
download or fetch completes. Legacy per-video folders and flat files
recorded on a video SHALL be protected the same way. Every file whose name starts with the base name of a Downloaded video's recorded filename followed by a dot SHALL also be protected while that video's subtitle state is Missing and it is not in the previous per-video-folder layout, because a subtitle file is placed before it is recorded. A file with any other extension under an episode base name, such as a stale `.mkv` or `.webm`, is not protected.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's show directory
- **THEN** the video's sync resets it to PENDING, resets its subtitle state to Missing and clears its recorded subtitle filename, clears its recorded filename, thumbnail filename, and quality, keeps its episode number, and schedules a thumbnail fetch and a fresh download of it

#### Scenario: Stale file of a video reset by the same pass
- **WHEN** a reconcile pass finds a Downloaded video whose recorded file is an `.mkv`, and its sync resets it for redownload
- **THEN** the same pass deletes the `.mkv`, no longer recorded on any video, including when the video has a recorded episode number and the `.mkv` is under its episode base name

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

#### Scenario: Files left by a Discarded video
- **WHEN** a reconcile pass finds the thumbnail `Season 1/S01E004 - Title.jpg` of a Discarded video whose episode number is recorded and whose filenames are cleared
- **THEN** the system deletes that file
