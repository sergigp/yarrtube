## Purpose

Automatically downloads a video via `yt-dlp` as soon as it is added to a tracked playlist, and tracks that video through the download lifecycle so its stored status always reflects whether it succeeded, is still being retried, or has permanently failed.

## Requirements

### Requirement: Download Triggered By Video Addition
The system SHALL automatically begin downloading a video as soon as it is newly added to a tracked playlist or a tracked channel, without requiring manual intervention.

#### Scenario: New video added to a tracked playlist
- **WHEN** a video is newly added to a tracked playlist
- **THEN** the system downloads that video without any manual action

#### Scenario: New video added to a tracked channel
- **WHEN** a video is newly added to a tracked channel (its `video_limit` most recent uploads)
- **THEN** the system downloads that video without any manual action

### Requirement: Video Status Reflects Download Progress
The system SHALL track a video's status through its download lifecycle: pending, in progress, downloaded, errored-but-retrying, permanently errored, or excluded. A video becomes excluded when a download attempt fails for a reason that can never succeed on a later attempt (see "Permanently Unavailable Videos Are Excluded"); excluded is a terminal status that is never retried or recovered.

#### Scenario: Download starts
- **WHEN** the system begins downloading a pending video
- **THEN** the video's status becomes in-progress

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's status becomes downloaded

#### Scenario: Download fails with attempts remaining
- **WHEN** a video download attempt fails and the system will still retry it
- **THEN** the video's status becomes errored-but-retrying

#### Scenario: Download fails permanently
- **WHEN** a video download has failed and the system will not retry it again
- **THEN** the video's status becomes permanently errored

#### Scenario: Download fails for a diagnosed permanent reason
- **WHEN** a video download attempt fails and the diagnosed reason (see "Failed Downloads Are Diagnosed") is one that can never succeed later — members-only content, a copyright/claimed-content block, a region block, a private video, a removed video, or a terminated account
- **THEN** the video's status becomes excluded, regardless of how many attempts remained

### Requirement: Automatic Retry On Download Failure
The system SHALL automatically retry a failed video download a bounded number of times before giving up on that attempt sequence, without any manual action. Exhausting that bounded sequence SHALL NOT prevent the video from being attempted again later by playlist or channel reconciliation. A failure that marks the video excluded (see "Permanently Unavailable Videos Are Excluded") SHALL NOT be retried within the attempt sequence and SHALL NOT be re-attempted later.

#### Scenario: Transient download failure
- **WHEN** a video download attempt fails and retries remain
- **THEN** the system automatically attempts the download again after a delay

#### Scenario: Retries exhausted
- **WHEN** a video download has failed on every allowed attempt
- **THEN** the system stops attempting that download within that attempt sequence, though the video may be attempted again later by playlist or channel reconciliation

#### Scenario: Permanently-unavailable failure is not retried
- **WHEN** a video download attempt fails for a permanently-unavailable reason
- **THEN** the system does not retry that download within the attempt sequence and does not re-attempt the video later

### Requirement: Download Failure Reason Is Recorded
The system SHALL record `yt-dlp`'s actual reported error text as a failed download attempt's failure reason, when `yt-dlp` reported one, instead of a generic message that does not distinguish one failure cause from another.

#### Scenario: yt-dlp reports a specific error
- **WHEN** a video download attempt fails and `yt-dlp` reported an error message on its standard error stream
- **THEN** that exact message is recorded as the attempt's failure reason

#### Scenario: yt-dlp fails without reporting a message
- **WHEN** a video download attempt fails and `yt-dlp` reported no error message
- **THEN** a generic failure reason is recorded instead

### Requirement: Failed Downloads Are Diagnosed
The system SHALL, whenever a video download attempt fails cleanly (`yt-dlp` ran and exited non-zero), determine a precise reason for the failure beyond the generic message the download itself reported, and record it so an operator can see why the download failed. The system SHALL obtain this reason even when the download's own error was generic (e.g. a bare "Video unavailable"). When no precise reason can be determined, the system SHALL record that the reason is undetermined rather than inventing one.

#### Scenario: A failed download's precise reason is recorded
- **WHEN** a video download attempt fails cleanly
- **THEN** the system determines and records a precise failure reason for that attempt

#### Scenario: A generic download error is still diagnosed
- **WHEN** a video download attempt fails with only a generic error (such as a bare "Video unavailable")
- **THEN** the system still determines and records the precise underlying reason when one is available

#### Scenario: No precise reason can be determined
- **WHEN** a video download attempt fails and no precise reason can be determined
- **THEN** the system records the reason as undetermined and does not treat the failure as permanent

### Requirement: Permanently Unavailable Videos Are Excluded
The system SHALL mark a video excluded, instead of errored, when the diagnosed reason for its failed download (see "Failed Downloads Are Diagnosed") is one that can never succeed on a later attempt. The reasons the system SHALL treat as permanently unavailable are: members-only content (requiring a paid channel membership `yt-dlp` cannot satisfy), a copyright or claimed-content block, a region/country block, a private video, a removed or no-longer-available video, and a video whose associated account has been terminated. An excluded video's download attempt SHALL be treated as a settled, non-failing outcome: the download task SHALL complete without error so the task queue neither retries nor dead-letters it. A failure whose diagnosed reason is NOT one of these — including an undetermined or merely generic reason (such as a bare "Video unavailable" with no further detail), or a network, timeout, HTTP server, rate-limit, bot-verification, or fragment/merge error — SHALL NOT be treated as permanently unavailable, and SHALL remain errored and subject to the normal retry and reconcile-recovery behavior.

#### Scenario: A diagnosed permanent reason excludes the video
- **WHEN** a video download attempt fails and the diagnosed reason is one of the permanently-unavailable reasons (members-only, copyright/claimed-content block, region block, private, removed, or terminated account)
- **THEN** the system marks the video excluded and completes the download task without a failure, so no retry is scheduled and the task is not dead-lettered

#### Scenario: A generic "Video unavailable" with no diagnosed cause is not excluded
- **WHEN** a video download attempt fails and the diagnosed reason is undetermined or merely a generic "Video unavailable" with no specific permanent cause
- **THEN** the system marks the video errored (retrying or permanently, per the retry policy) and does not exclude it, so it remains eligible for retry and 24h reconcile recovery

#### Scenario: A recoverable failure is not excluded
- **WHEN** a video download attempt fails for a transient reason such as a network, timeout, HTTP server, rate-limit, bot-verification, or fragment error
- **THEN** the system marks the video errored (retrying or permanently, per the retry policy) and does not exclude it

### Requirement: Per-Playlist Output Directory
The system SHALL save a downloaded video's file inside the `Season <season>` folder of its show directory, the show directory being determined by its owning playlist's or channel's configured storage path within a single configured root directory. A storage path may contain multiple segments, producing nested subdirectories. The file's name SHALL be the video's episode base name (see `video-naming` and `tv-show-layout`) with the media extension, so the video's file, its thumbnail, and its metadata file sit next to each other under one base name. The download SHALL reuse the episode number already recorded on the video (a playlist video's from when it was first stored, or one recorded by an earlier thumbnail-ahead fetch, see `video-thumbnails`), and SHALL assign a channel video's number when it has none yet.

#### Scenario: Video downloaded
- **WHEN** a video belonging to a playlist finishes downloading
- **THEN** its file is saved as `Season <season>/<episode base>.mp4` under the configured root directory, in the subdirectory (or nested subdirectories) identified by its playlist's storage path

#### Scenario: Video downloaded for a channel
- **WHEN** a video belonging to a channel finishes downloading
- **THEN** its file is saved as `Season <season>/<episode base>.mp4` under the configured root directory, in the subdirectory (or nested subdirectories) identified by its channel's storage path

#### Scenario: Root directory not configured
- **WHEN** no root output directory has been explicitly configured
- **THEN** the system uses a default root directory

#### Scenario: Video downloaded under a legacy flat layout
- **WHEN** a video was downloaded before the TV-show layout was introduced and its recorded file still exists at its original location (a per-video folder or a flat file under the source directory)
- **THEN** the system continues to treat that file as valid, keeps serving it, and leaves moving it to the `migrate-layout` subcommand

#### Scenario: Video's thumbnail was already fetched ahead of its download
- **WHEN** a video's full download runs and that video already has a recorded episode number and thumbnail filename from an earlier thumbnail-only fetch
- **THEN** the system saves the video's file under that same episode base name, next to the thumbnail

#### Scenario: Video has no pre-fetched thumbnail
- **WHEN** a channel video's full download runs and that video has no recorded episode number
- **THEN** the system assigns its number from the publish timestamp the downloader reports and saves the file under the resulting episode base name

#### Scenario: Playlist video downloaded
- **WHEN** a playlist video's full download runs
- **THEN** the system saves the file under the episode number recorded when the playlist first stored the video, whatever the reported publish timestamp

#### Scenario: Finished download cannot be placed
- **WHEN** numbering, moving or recording a finished download fails
- **THEN** the system discards the staged files and marks the video errored like a failed download (retrying, or errored on the last attempt), keeping any episode number already assigned

### Requirement: Download Skipped For a Deleted Playlist
The system SHALL NOT attempt to download a video, or treat it as an error, when the download is attempted after the video's owning playlist or channel no longer exists.

#### Scenario: Playlist deleted before its video's download runs
- **WHEN** a video's download is attempted for a playlist that no longer exists
- **THEN** the system makes no download attempt and does not change that video's status

#### Scenario: Channel deleted before its video's download runs
- **WHEN** a video's download is attempted for a channel that no longer exists
- **THEN** the system makes no download attempt and does not change that video's status

### Requirement: Download Skipped For a Removed Video
The system SHALL NOT attempt to download a video, or treat it as an error, when the download is attempted after that video's own record no longer exists.

#### Scenario: Video removed before its own download runs
- **WHEN** a video's download is attempted but that video is no longer stored
- **THEN** the system makes no download attempt

### Requirement: Download Applies Playlist Quality
The system SHALL download a video's file using the resolution associated with its owning playlist's or channel's configured quality tier (`high`, `mid`, or `low`), preferring an mp4 container with h264 video and aac audio at every tier, and SHALL fall back to the best available stream within that resolution instead of failing the download outright when no stream matches the mp4/h264/aac preference.

#### Scenario: Playlist configured for high quality
- **WHEN** a video belonging to a playlist configured with quality `high` is downloaded
- **THEN** the system downloads it with no resolution cap, preferring an mp4/h264/aac stream

#### Scenario: Playlist configured for mid quality
- **WHEN** a video belonging to a playlist configured with quality `mid` is downloaded
- **THEN** the system downloads it capped at 720p, preferring an mp4/h264/aac stream

#### Scenario: Playlist configured for low quality
- **WHEN** a video belonging to a playlist configured with quality `low` is downloaded
- **THEN** the system downloads it capped at 480p, preferring an mp4/h264/aac stream

#### Scenario: Channel configured for a quality tier
- **WHEN** a video belonging to a channel configured with a given quality tier is downloaded
- **THEN** the system applies that tier's resolution cap and mp4/h264/aac preference the same way it does for a playlist

#### Scenario: No stream matches the preferred container/codec
- **WHEN** a video has no available stream in mp4/h264/aac at or below its owning playlist's or channel's resolution cap
- **THEN** the system downloads the best available stream within that resolution cap instead of failing the download

### Requirement: Downloaded Quality Is Recorded
The system SHALL record, on the video itself, the quality tier (`high`, `mid`, or `low`) it was actually downloaded at and the video height (in pixels) of the downloaded file, when the downloader reports it, whenever a download succeeds. A video that has not yet completed a successful download SHALL have no recorded quality or height. A download SHALL settle the video's quality (see `late-video-data`) when the video is old, or when the downloaded height reaches its tier's settlement height (`high` 1080, `mid` 720, `low` 480; `high` downloads stay uncapped, 1080 is only where its checks stop); otherwise the quality SHALL stay unsettled, last checked at the download time. A download the downloader reports as degraded by YouTube's SABR-only streaming experiment SHALL instead count as a failed quality check: its tier and height are recorded as usual, but its quality settles only as a failed check does (an old video with at least 3 failed attempts). Videos downloaded before heights were recorded SHALL have their quality settled.

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully at a given quality tier with a 1080-pixel-high file
- **THEN** the video's recorded quality becomes that tier and its recorded height 1080

#### Scenario: Young video downloaded below its cap
- **WHEN** a video published 1 hour ago, owned by a `mid` playlist, is downloaded at 360 pixels high
- **THEN** its quality is unsettled, last checked at the download time

#### Scenario: Young video downloaded at its cap
- **WHEN** a video published 1 hour ago, owned by a `mid` playlist, is downloaded at 720 pixels high
- **THEN** its quality is settled

#### Scenario: Young high video downloaded at 1080
- **WHEN** a video published 1 hour ago, owned by a `high` channel, is downloaded at 1080 pixels high
- **THEN** its quality is settled

#### Scenario: Old video downloaded
- **WHEN** a video published 30 days ago is downloaded at any height, not reported as degraded
- **THEN** its quality is settled

#### Scenario: Old video downloaded degraded
- **WHEN** a video published 30 days ago is downloaded at 360 pixels high and the downloader reports the SABR-only streaming signal
- **THEN** its tier and height 360 are recorded, its quality stays unsettled and one failed attempt is counted, so a later quality check tries again

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded quality or height

#### Scenario: Library upgraded
- **WHEN** the system starts on a database with Downloaded videos stored before heights were recorded
- **THEN** each such video has no recorded height and its quality is settled

### Requirement: Downloaded Filename Is Recorded
The system SHALL record, on the video itself, the exact filename its file
was saved under on disk, whenever a download succeeds. A video that has not
yet completed a successful download SHALL have no recorded filename.

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's recorded filename becomes the exact name of the file that was saved to disk

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded filename

#### Scenario: Video re-downloaded after being reset
- **WHEN** a video that previously had a recorded filename is reset and successfully downloaded again
- **THEN** its recorded filename becomes the filename of the new download, replacing the prior one

### Requirement: Downloaded Thumbnail Is Recorded
The system SHALL record, on the video itself, the exact filename of the
thumbnail file that was saved to disk alongside it, whenever a download
succeeds and a thumbnail file was written for it. A video that has not yet
completed a successful download, or whose download succeeded without a
thumbnail being written, SHALL have no recorded thumbnail filename.

#### Scenario: Download succeeds with a thumbnail
- **WHEN** a video download completes successfully and a thumbnail file was written for it
- **THEN** the video's recorded thumbnail filename becomes the exact name of that thumbnail file

#### Scenario: Download succeeds without a thumbnail
- **WHEN** a video download completes successfully but no thumbnail file was written for it
- **THEN** the video has no recorded thumbnail filename

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded thumbnail filename

#### Scenario: Video re-downloaded after being reset
- **WHEN** a video that previously had a recorded thumbnail filename is reset and successfully downloaded again
- **THEN** its recorded thumbnail filename reflects the new download (a fresh filename, or none, replacing the prior one)

### Requirement: Downloaded Duration Is Recorded
The system SHALL record, on the video itself, its duration in seconds, whenever a download succeeds and a duration is available for it. A video that has not yet completed a successful download SHALL have no recorded duration.

#### Scenario: Download succeeds with a known duration
- **WHEN** a video download completes successfully and a duration is available for it
- **THEN** the video's recorded duration becomes that duration, in seconds

#### Scenario: Download succeeds without a known duration
- **WHEN** a video download completes successfully but no duration could be determined for it
- **THEN** the video has no recorded duration

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded duration

#### Scenario: Video re-downloaded after being reset
- **WHEN** a video that previously had a recorded duration is reset and successfully downloaded again
- **THEN** its recorded duration reflects the new download (a fresh duration, or none, replacing the prior one)

### Requirement: Sync Time Is Recorded
The system SHALL record, on the video itself, the time its download last succeeded (its sync time). A video that has not completed a successful download since it was created or last reset for redownload SHALL have no recorded sync time. Recording a thumbnail, a watch-state change, or any other update SHALL NOT change a video's sync time.

#### Scenario: Download succeeds
- **WHEN** a video download completes successfully
- **THEN** the video's recorded sync time becomes the time the download completed

#### Scenario: Video not yet successfully downloaded
- **WHEN** a video is pending, in progress, or has only failed attempts so far
- **THEN** the video has no recorded sync time

#### Scenario: Video reset for redownload
- **WHEN** a downloaded video is reset for redownload
- **THEN** the video has no recorded sync time until its next download succeeds

#### Scenario: Thumbnail recorded after download
- **WHEN** a thumbnail is recorded for a video that was already downloaded
- **THEN** the video's recorded sync time is unchanged

#### Scenario: Video downloaded before sync time was tracked
- **WHEN** the system upgrades storage that holds videos already downloaded
- **THEN** each of those videos gets its last update time as its recorded sync time, and every other video has none

### Requirement: Concurrent Downloads Use Distinct Folders
The system SHALL give every video its own episode base name even when
downloads of different videos run at the same time. Two videos downloading
concurrently into the same show SHALL NOT end up sharing a base name, even
when their titles are identical and they were published on the same day,
because a channel's same-day indexes are assigned from the recorded numbers
and a playlist's numbers from its never-reused counter.

#### Scenario: Two videos with the same title download at the same time
- **WHEN** two different videos with the same title and publish day, in the same playlist or channel, are downloaded concurrently
- **THEN** each video's file, thumbnail, and metadata are saved under a different episode base name, and each video's recorded filename points at its own file

### Requirement: Download Finished For a Deleted Video Leaves No Files
The system SHALL remove the files a download created (its media file,
thumbnail and NFO) when, by the time that download finishes, the video's
record (or its owning playlist or channel) no longer exists. A deletion that
happens while a download is running SHALL NOT leave behind files that no
stored video accounts for. Season folders and show files are left alone.

#### Scenario: Playlist deleted while one of its videos is downloading
- **WHEN** a playlist is deleted, and its output directory removed, while one of its videos is still downloading
- **THEN** once that download finishes, the system removes the files the download wrote, and does not record the download on any video

#### Scenario: Video removed from its playlist while downloading
- **WHEN** a video's record is removed while that video is still downloading
- **THEN** once that download finishes, the system removes the files the download wrote

#### Scenario: Video still exists when its download finishes
- **WHEN** a download finishes and the video's record still exists
- **THEN** the system records the download as usual and keeps its files

### Requirement: SABR-Only Streaming Is Surfaced
The system SHALL detect, during a video download attempt, when `yt-dlp`
reports that YouTube's SABR-only streaming experiment is active for the
session — the condition under which the better formats are skipped because
they carry no downloadable URL. When that condition is detected, the system
SHALL emit a distinct warn-level event that an operator can recognize and
search for without reading surrounding output, carrying the affected video's
id and the reason text `yt-dlp` reported as structured fields (per the
`logging` capability's conventions). The system SHALL surface this condition
whether the download then failed outright or completed by falling back to a
lower-quality format, so that a merely degraded download is flagged and not
mistaken for a healthy one. When `yt-dlp` does not report the SABR-only
signal, the system SHALL NOT emit the event. Detecting and surfacing the
condition SHALL NOT, by itself, change which formats or clients the download
requests, the recorded failure reason, or the video's status and retry
behavior.

#### Scenario: Download fails under SABR-only streaming
- **WHEN** a video download attempt fails and `yt-dlp` reported the SABR-only
  streaming signal (the better formats were skipped as missing a URL)
- **THEN** the system emits a distinct warn-level SABR event with the video id
  and the reported reason as fields

#### Scenario: Download degrades to a lower-quality format under SABR-only streaming
- **WHEN** a video download attempt completes successfully but `yt-dlp`
  reported the SABR-only streaming signal, having skipped the better formats
  and fallen back to a lower-quality one
- **THEN** the system emits the same distinct warn-level SABR event with the
  video id and the reported reason as fields, in addition to recording the
  download as succeeded

#### Scenario: Download without the SABR-only signal
- **WHEN** a video download attempt runs and `yt-dlp` does not report the
  SABR-only streaming signal
- **THEN** the system does not emit the SABR event

#### Scenario: Surfacing does not alter download outcome
- **WHEN** the SABR-only streaming signal is detected during a download attempt
- **THEN** the video's recorded status, failure reason, and retry/exclusion
  behavior are exactly what they would have been had the signal only been
  logged, and no change is made to the formats or clients requested

### Requirement: Download Skipped For an Already-Settled Video
The system SHALL, when a download task runs for a video whose download status is already Downloaded, Excluded or Errored, complete the task without downloading and without changing the video or its files.

#### Scenario: Download task for an already-downloaded video
- **WHEN** a download task runs for a video whose status is Downloaded
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for an excluded video
- **WHEN** a download task runs for a video whose status is Excluded
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

#### Scenario: Download task for a permanently errored video
- **WHEN** a download task runs for a video whose status is Errored
- **THEN** no download is attempted, the video is left unchanged, and the task completes without error

### Requirement: Young Video Quality Upgrade
The system SHALL run a quality check for a Downloaded video whose quality is unsettled: it SHALL ask the downloader which height the video's owning playlist's or channel's tier would download now, without downloading. When that height is above the recorded height, the system SHALL download the video again at that tier and replace its media file in place: same episode file name, same episode number, keeping its thumbnail, subtitles, chapters, metadata and watch state, and recording the new height and tier. The replaced file SHALL count as downloaded again for media-server scanning. Each quality check SHALL record its time as the quality's last check and settle the quality under the `late-video-data` policy: settled when the new height reaches the tier's settlement height or the video is old, unsettled otherwise. The media file SHALL be replaced only when the re-download's height is above the recorded one; otherwise the re-download SHALL be discarded and counted as a failed attempt. A probe or re-download the downloader reports as degraded by the SABR-only streaming experiment SHALL count as a failed attempt rather than a conclusive result; a degraded re-download that is still taller SHALL replace the file. A failed check or failed re-download SHALL keep the existing file and recorded fields unchanged, count one failed attempt, and SHALL NOT fail the task or be retried by the task queue. A quality check SHALL make no changes for a video that is not Downloaded, whose quality is settled, or whose record no longer exists.

#### Scenario: HD available after processing
- **WHEN** a quality check runs for a `high` video published 1 day ago, recorded at 360 pixels high, and the downloader reports 1080 available
- **THEN** the video is downloaded again, its file is replaced under the same name, its recorded height becomes 1080, its subtitles, chapters and watch position are unchanged, and its quality is settled (1080 reaches the `high` settlement height)

#### Scenario: Nothing better yet
- **WHEN** a quality check runs for a young video recorded at 360 pixels high and the downloader reports 360 available
- **THEN** nothing is downloaded, the check time is recorded and the quality stays unsettled

#### Scenario: Video turned old
- **WHEN** a quality check runs for a video published 8 days ago, recorded at 720 pixels high, and the downloader reports 720 available
- **THEN** nothing is downloaded and the quality is settled

#### Scenario: Re-download fails
- **WHEN** a quality check finds a higher height but the re-download fails
- **THEN** the existing file and recorded fields are unchanged, one failed attempt is counted, and the task completes without error

#### Scenario: Re-download no better
- **WHEN** a quality check for a video recorded at 360 pixels high finds 1080 available but the re-download lands at 240 pixels high (YouTube served it a worse format list)
- **THEN** the existing file and recorded height are kept, the re-download is discarded, one failed attempt is counted, and the task completes without error

#### Scenario: Degraded probe
- **WHEN** a quality check for a video published 30 days ago, recorded at 360 pixels high, probes 360 available while the downloader reports the SABR-only streaming signal
- **THEN** nothing is downloaded, one failed attempt is counted and the quality stays unsettled

#### Scenario: Degraded re-download still taller
- **WHEN** a quality check for a video published 8 days ago, recorded at 360 pixels high, finds 1080 available and the re-download lands at 720 with the SABR-only streaming signal
- **THEN** the file is replaced, the recorded height becomes 720, one failed attempt is counted and the quality stays unsettled

#### Scenario: Video reset while the upgrade downloads
- **WHEN** a video is reset for redownload or deleted while its quality upgrade is downloading
- **THEN** the upgrade places no file and records nothing
