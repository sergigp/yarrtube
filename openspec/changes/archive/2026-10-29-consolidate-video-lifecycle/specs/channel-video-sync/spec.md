# Spec Delta

## MODIFIED Requirements

### Requirement: Filesystem Reconciliation Against Recorded Downloads (Channels)
The system SHALL, for every channel, compare the files present in that
channel's show directory (recursively through its season folders) against
the recorded filename and recorded thumbnail filename of each of its
Downloaded videos, and heal any divergence it finds, the same way
`playlist-reconciliation`'s filesystem reconciliation does for playlists:
a Downloaded video whose recorded file is missing or not mp4 is reset by its
video sync (see `video-sync`), and every file left unaccounted for once every
video of the pass has been synced is deleted,
with the same protected set: show files, season folders, every recorded filename, thumbnail filename and subtitle filename of any video regardless of status, the NFO
sharing a recorded filename's base name, every media (`.mp4`), thumbnail (`.jpg`) or NFO (`.nfo`) file under the
episode base name of a video whose number is recorded but whose download or
thumbnail fetch has not completed yet, and legacy per-video folders or flat files
recorded on a video. Every file whose name starts with the base name of a Downloaded video's recorded filename followed by a dot SHALL also be protected while that video's subtitle state is Missing and it is not in the previous per-video-folder layout, because a subtitle file is placed before it is recorded. A file with any other extension under an episode base name, such as a stale `.mkv` or `.webm`, is not protected.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its channel's show directory
- **THEN** the video's sync resets it to PENDING, resets its subtitle state to Missing and clears its recorded subtitle filename, clears its recorded filename, thumbnail filename and quality, keeps its episode number, and schedules a thumbnail fetch and a fresh download of it

#### Scenario: Downloaded video's file is present but not mp4
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's show directory but is not an mp4 container
- **THEN** the video's sync resets it to PENDING, clears its recorded filename and quality, and schedules a thumbnail fetch and a fresh download of it, and the same pass deletes the stale file, no longer recorded on any video, including when the video has a recorded episode number and the file is under its episode base name

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

## REMOVED Requirements

### Requirement: Permanently Failed Video Recovery (Channels)
**Reason**: Per-video lifecycle work moves into the video sync, which every reconcile pass runs for every stored video.
**Migration**: See `video-sync` "Sync Recovers a Permanently Failed Video" and "Reconcile Passes Sync Stored Videos".

### Requirement: Missing Metadata Recovery (Channels)
**Reason**: Per-video lifecycle work moves into the video sync, which every reconcile pass runs for every stored video.
**Migration**: See `video-sync` "Sync Repairs Missing Metadata" and "Reconcile Passes Sync Stored Videos".

### Requirement: Missing Thumbnail Recovery (Channels)
**Reason**: The video sync already decides the thumbnail fetch; the pass-specific exceptions no longer apply since every stored video goes through the same sync.
**Migration**: See `video-sync` "Sync Schedules the Thumbnail Fetch" and "Reconcile Passes Sync Stored Videos".

### Requirement: Stranded Video Recovery (Channels)
**Reason**: The video sync already schedules the download of every unsettled video and now warns when it was stranded.
**Migration**: See `video-sync` "Sync Schedules the Download", "Sync Warns About a Stranded Video" and "Reconcile Passes Sync Stored Videos".

### Requirement: Missing Subtitles Recovery (Channels)
**Reason**: The video sync already decides the subtitle fetch; the pass-specific exceptions no longer apply.
**Migration**: See `video-sync` "Sync Schedules the Subtitle Fetch" and "Reconcile Passes Sync Stored Videos".
