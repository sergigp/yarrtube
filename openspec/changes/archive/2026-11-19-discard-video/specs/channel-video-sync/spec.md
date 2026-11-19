# Spec Delta

## MODIFIED Requirements

### Requirement: Channel Video Discovery Via yt-dlp
The system SHALL discover a channel's current videos by invoking `yt-dlp`
against that channel's videos listing, requesting only its `video_limit`
most recent uploads plus one more for each of the channel's stored videos
whose download is Unavailable or Discarded and each discarded upload the
channel remembers (skipped videos don't count toward the limit; see
`video-discard`), and parsing each discovered video's ID and title from
structured (JSON) output rather than delimited text, so that a video title
containing arbitrary characters (including a `|` or other delimiter-like
character) cannot corrupt discovery of that video or any other.

#### Scenario: Channel has more uploads than its video limit
- **WHEN** a reconcile pass discovers a channel's videos
- **THEN** the system requests and receives at most `video_limit` videos plus the number of the channel's skipped videos, ordered most recent first

#### Scenario: A discovered video's title contains a delimiter-like character
- **WHEN** a reconcile pass discovers a video whose title contains a `|` character (or other text that would corrupt a delimited-text parse)
- **THEN** the system correctly records that video's exact title and ID, and no other discovered video's data is corrupted by it

#### Scenario: yt-dlp fails to list a channel's videos
- **WHEN** a recurring or on-demand reconcile pass's attempt to discover a channel's current videos fails (e.g. the channel is private, terminated, or `yt-dlp` errors)
- **THEN** the system does not alter any of that channel's stored videos, logs the failure, and does not treat it as a fatal error for the reconcile pass

#### Scenario: Skipped videos widen the request
- **WHEN** a reconcile pass discovers the videos of a channel with `video_limit` 3 that has one Unavailable and one Discarded stored video
- **THEN** the system requests at most 5 videos

### Requirement: New Video Persistence (Channels)
The system SHALL store each video discovered for a channel that is not
already stored for that channel and not remembered as discarded by it
(see `video-discard`), with a status of PENDING and its current
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

#### Scenario: Channel's most recent uploads include a remembered discarded video
- **WHEN** a reconcile pass for a channel finds among its current most-recent uploads a video the channel remembers as discarded
- **THEN** the system doesn't store it and publishes no `VideoAddedToChannel` event for it

### Requirement: Aged-Out Video Eviction (Channels)
The system SHALL delete a stored video's record when a reconcile pass for a
channel finds that it is no longer among the channel's current most recent uploads up to and
including the `video_limit`-th one whose download is not Unavailable or
Discarded and that the channel doesn't remember as discarded — whether
because it was deleted/unlisted on YouTube or because newer uploads have
pushed it past that position — SHALL remember an evicted Discarded video's
YouTube id for the channel before deleting its record, SHALL log
each such eviction, and SHALL publish a `VideoRemovedFromChannel` event
carrying that video's title, its recorded filename (if any), and whether it
had been downloaded.

#### Scenario: Previously stored video no longer among the channel's most recent uploads
- **WHEN** a reconcile pass finds that a video previously stored for a channel is no longer among that channel's current `video_limit` most recent uploads
- **THEN** the system deletes its stored record and logs the eviction

#### Scenario: Eviction publishes a VideoRemovedFromChannel event
- **WHEN** a reconcile pass evicts a stored video's record for a channel
- **THEN** the system publishes a `VideoRemovedFromChannel` event containing the channel's handle, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

#### Scenario: A skipped video doesn't take a slot
- **WHEN** a reconcile pass for a channel with `video_limit` 3 lists uploads A, B, C, D (most recent first) and B is stored as Unavailable
- **THEN** A, B, C and D are all kept, so the channel has 3 videos that aren't skipped

#### Scenario: Evicting a discarded video remembers it
- **WHEN** a reconcile pass for a channel evicts a stored video whose download is Discarded
- **THEN** the channel remembers its YouTube id, then its stored record is deleted

#### Scenario: Window ends at the limit-th unskipped video
- **WHEN** a reconcile pass for a channel with `video_limit` 2 lists uploads A, B, C (most recent first), none skipped, and C was stored
- **THEN** C is evicted

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
episode base name of a video whose number is recorded, that is not Discarded, but whose download or
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

#### Scenario: Files left by a Discarded video
- **WHEN** a reconcile pass finds the thumbnail `Season 2026/S2026E031601 - Title.jpg` of a Discarded video whose episode number is recorded and whose filenames are cleared
- **THEN** the system deletes that file
