## MODIFIED Requirements

### Requirement: Reconcile On Channel Creation
The system SHALL run one reconcile pass for a channel as soon as practical after it is created, and SHALL write the channel's show files (`tvshow.nfo` and, when it has a stored avatar, `poster.jpg`; see `tv-show-layout`) in that pass. If discovering the channel's current videos fails, that pass SHALL fail, so it is retried with the creation event's processing rather than leaving the new channel empty until a recurring pass.

#### Scenario: Newly created channel gets reconciled
- **WHEN** a channel is successfully created
- **THEN** the system writes its show files, discovers its current most-recent videos (up to `video_limit`) and persists them

#### Scenario: Discovery fails during the initial reconcile
- **WHEN** the initial reconcile pass of a newly created channel fails to discover the channel's current videos
- **THEN** the system stores no video, publishes no event, schedules no recurring reconcile of the channel, and fails the `ChannelCreated` event's processing so it is retried

### Requirement: Filesystem Reconciliation Against Recorded Downloads (Channels)
The system SHALL, for every channel, compare the files present in that
channel's show directory (recursively through its season folders) against
the recorded filename and recorded thumbnail filename of each of its
Downloaded videos, and heal any divergence it finds, the same way
`playlist-reconciliation`'s filesystem reconciliation does for playlists,
with the same protected set: show files, season folders, every recorded
filename and thumbnail filename of any video regardless of status, the NFO
sharing a recorded filename's base name, every file sharing the episode base
name of a video whose number is recorded but whose download or thumbnail
fetch has not completed yet, and legacy per-video folders or flat files
recorded on a video.

#### Scenario: Downloaded video's file is missing from disk
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is not present in its channel's show directory
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, keeps its episode number, and triggers a fresh download of it

#### Scenario: Downloaded video's file is present but not mp4
- **WHEN** a reconcile pass finds that a Downloaded video's recorded file is present in its channel's show directory but is not an mp4 container
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: A file on disk is not accounted for by any Downloaded video
- **WHEN** a reconcile pass finds a file in the channel's show directory or one of its season folders that is not a show file, not a recorded filename or recorded thumbnail filename or NFO of any video, not a downloader-managed in-progress temporary file, and does not share the episode base name of a video whose number is recorded
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
