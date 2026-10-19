# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
