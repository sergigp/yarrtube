## MODIFIED Requirements

### Requirement: Filesystem Reconciliation Against Recorded Downloads
The system SHALL, for every playlist regardless of kind, compare the files
present in that playlist's output directory against the recorded filename
and recorded thumbnail filename of each of its Downloaded videos, and heal
any divergence it finds. It SHALL also protect any video's (regardless of
status) recorded thumbnail folder from being swept as orphaned, since a
thumbnail may have been fetched for a video that has not yet been
downloaded. It SHALL also
protect every folder that a download or thumbnail fetch of a video of that
playlist may be writing into while that video's folder is not recorded
yet (the video is not Downloaded), whatever its status and even when it has
no recorded thumbnail, because such a folder is only recorded once the
download or fetch completes. When the same pass changes such a video's
title, it SHALL also protect the folders a download started under the
previous title may be writing into.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its playlist's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename, thumbnail filename, and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the playlist's output directory that is not the recorded filename or recorded thumbnail filename of any currently Downloaded video, is not a downloader-managed in-progress temporary file, and is not a folder protected for a video whose folder is not recorded yet
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

#### Scenario: A thumbnail fetch without a recorded folder is in progress
- **WHEN** a reconcile pass runs while a Pending video of the playlist has no recorded thumbnail filename, and the folder its thumbnail fetch is writing into is present on disk
- **THEN** the system does not treat that folder as orphaned and does not delete it

#### Scenario: Video renamed while its download is in progress
- **WHEN** a reconcile pass changes the title of an In Progress video of the playlist, and the folder its download started writing into under the previous title is present on disk
- **THEN** the system records the new title, and does not treat the folder named after the previous title as orphaned
