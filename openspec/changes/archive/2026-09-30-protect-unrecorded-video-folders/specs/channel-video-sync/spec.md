## MODIFIED Requirements

### Requirement: Filesystem Reconciliation Against Recorded Downloads (Channels)
The system SHALL, for every channel, compare the files present in that
channel's output directory against the recorded filename of each of its
Downloaded videos, and heal any divergence it finds, the same way
`playlist-reconciliation`'s filesystem reconciliation does for playlists.
It SHALL also protect any video's (regardless of status) recorded
thumbnail folder from being swept as orphaned, since a thumbnail may have
been fetched for a video that has not yet been downloaded. It SHALL also
protect every folder that a download or thumbnail fetch of a video of that
channel may be writing into while that video's folder is not recorded
yet (the video is not Downloaded), whatever its status and even when it has
no recorded thumbnail, because such a folder is only recorded once the
download or fetch completes. When the same pass changes such a video's
title, it SHALL also protect the folders a download started under the
previous title may be writing into.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its channel's output directory
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Downloaded video's file is present but not mp4
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's output directory but is not an mp4 container
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the channel's output directory that is not the recorded filename of any currently Downloaded video, is not a downloader-managed in-progress temporary file, and is not a folder protected for a video whose folder is not recorded yet
- **THEN** the system deletes that file

#### Scenario: Downloaded video's file is present
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's output directory and is an mp4 container
- **THEN** the system takes no action for that video

#### Scenario: A pending video's pre-fetched thumbnail folder is present
- **WHEN** a reconcile pass finds a folder on disk that matches a Pending or In Progress video's recorded thumbnail filename
- **THEN** the system does not treat that folder as orphaned and does not delete it

#### Scenario: A download without a pre-fetched thumbnail is in progress
- **WHEN** a reconcile pass runs while a video of the channel is In Progress with no recorded thumbnail filename, and the folder its download is writing into is present on disk
- **THEN** the system does not treat that folder as orphaned and does not delete it

#### Scenario: A thumbnail fetch without a recorded folder is in progress
- **WHEN** a reconcile pass runs while a Pending video of the channel has no recorded thumbnail filename, and the folder its thumbnail fetch is writing into is present on disk
- **THEN** the system does not treat that folder as orphaned and does not delete it

#### Scenario: Video renamed while its download is in progress
- **WHEN** a reconcile pass changes the title of an In Progress video of the channel, and the folder its download started writing into under the previous title is present on disk
- **THEN** the system records the new title, and does not treat the folder named after the previous title as orphaned
