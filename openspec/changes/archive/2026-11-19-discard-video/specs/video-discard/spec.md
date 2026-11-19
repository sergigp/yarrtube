# Spec Delta

## Purpose

Lets the user stop a video from ever being downloaded (e.g. one too big for the container to download) or get rid of a downloaded one: it is never attempted again, its files are deleted and it is hidden everywhere. Irreversible.

## ADDED Requirements

### Requirement: Discard A Video
The system SHALL let a client discard a video (`POST /api/videos/{id}/discard`, answering 204) whose download is Pending, Retrying, Failed or Downloaded. Its download SHALL become Discarded with no failed attempts, no failure reason and no recorded file, thumbnail or subtitle file, and every task of the video pending in the queue SHALL be removed. Discarding a Discarded video SHALL answer 204 and change nothing.

#### Scenario: Discard a retrying download
- **WHEN** a client discards a video whose download is Retrying with 3 failed attempts and a download task pending in 6 hours
- **THEN** the request answers 204, its download is Discarded with no failed attempts and no failure, and no task of the video is pending

#### Scenario: Discard a pending video
- **WHEN** a client discards a video whose download is Pending with a thumbnail fetch and a download pending
- **THEN** its download is Discarded and neither task is pending

#### Scenario: Discard a failed download
- **WHEN** a client discards a video whose download is Failed with 11 failed attempts
- **THEN** its download is Discarded with no failed attempts

#### Scenario: Discard twice
- **WHEN** a client discards a video whose download is already Discarded
- **THEN** the request answers 204 and the video is unchanged

### Requirement: Discarding A Video Deletes Its Files
Discarding a video SHALL delete every file it has recorded from its show folder: a Downloaded video's media file, the NFO next to it, its thumbnail and its subtitle file; a video not downloaded yet, its fetched thumbnail. Files are deleted the same way as for a video evicted from its source, keeping the show files and season folder. Discarding a video with no recorded file SHALL delete nothing.

#### Scenario: Discard a downloaded episode
- **WHEN** a client discards a Downloaded channel video recorded as `Season 2026/S2026E031501 - Title.mp4` with its thumbnail and subtitles
- **THEN** a file deletion of the video is scheduled to run now and, once it ran, `S2026E031501 - Title.mp4`, `.nfo`, `.jpg` and its subtitle file are gone from the channel's `Season 2026` folder

#### Scenario: Discard a video never downloaded with its thumbnail
- **WHEN** a client discards a video whose download is Retrying and whose thumbnail was fetched as `Season 2026/S2026E031601 - Title.jpg`
- **THEN** a file deletion of the video is scheduled to run now and, once it ran, that thumbnail is gone

#### Scenario: Discard a video with no file
- **WHEN** a client discards a video whose download is Retrying and that has no thumbnail
- **THEN** no file deletion is scheduled

### Requirement: Only A Video Not Being Downloaded Can Be Discarded
Discarding a video whose download is In Progress or Unavailable SHALL answer 409 and change nothing. Discarding an unknown video SHALL answer 404.

#### Scenario: Download in progress
- **WHEN** a client discards a video whose download is In Progress
- **THEN** the request answers 409 and the video and its tasks are unchanged

#### Scenario: Unavailable
- **WHEN** a client discards a video whose download is Unavailable
- **THEN** the request answers 409

#### Scenario: Unknown video
- **WHEN** a client discards a video id that doesn't exist
- **THEN** the request answers 404

### Requirement: A Discarded Video Is Never Attempted
No step SHALL be attempted for a Discarded video: a video sync SHALL schedule nothing for it, and a download, subtitle, chapters, quality, summary or categorization task that runs for it SHALL do nothing and complete without being re-queued. A step that was already running when the video was discarded SHALL record nothing on it when it finishes. A channel or playlist reconcile that still finds the video SHALL keep it Discarded.

#### Scenario: Video sync
- **WHEN** a video sync runs for a Discarded video with no thumbnail
- **THEN** no task of the video is scheduled

#### Scenario: Stale download task
- **WHEN** a download task runs for a Discarded video
- **THEN** nothing is downloaded, the video stays Discarded and the task completes without being re-queued

#### Scenario: Quality upgrade finishing after the discard
- **WHEN** a quality upgrade of a Downloaded video is running, the video is discarded, and the upgrade then places a better file
- **THEN** the video stays Discarded with no recorded file

#### Scenario: Channel resync
- **WHEN** a channel reconcile finds a Discarded video still among the channel's uploads, under a new title
- **THEN** the video's title is updated and it stays Discarded with no download scheduled

### Requirement: A Discarded Video Stays Discarded After Leaving Its Source
When a reconcile removes a Discarded video from its source (evicted from a channel's window, or no longer in a YouTube-linked playlist), the source SHALL remember the video's YouTube id. A later reconcile of that source SHALL NOT store a remembered video again, and a channel SHALL count a remembered upload as skipped (see `channel-video-sync`). Deleting the source SHALL forget its remembered videos.

#### Scenario: Channel window reaches back to an evicted discarded video
- **WHEN** a channel with `video_limit` 2 evicts its Discarded upload C, and a later reconcile runs after the limit is raised to 3
- **THEN** C is not stored again, no `VideoAddedToChannel` is published for it, and it doesn't count toward the limit

#### Scenario: Discarded video re-added to a playlist
- **WHEN** a Discarded video leaves a YouTube-linked playlist, is removed by a reconcile, and is later added back to the playlist on YouTube
- **THEN** the next reconcile doesn't store it again and publishes no `VideoAddedToPlaylist` for it

#### Scenario: Source deleted and tracked again
- **WHEN** a channel with a remembered discarded video is deleted and then tracked again
- **THEN** the video is treated as any new upload
