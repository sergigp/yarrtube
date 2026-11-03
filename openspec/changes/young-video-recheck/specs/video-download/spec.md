## MODIFIED Requirements

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

## ADDED Requirements

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
