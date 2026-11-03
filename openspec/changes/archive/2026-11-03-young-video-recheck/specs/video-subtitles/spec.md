## MODIFIED Requirements

### Requirement: Subtitle State
Every video SHALL record a subtitle state: Missing (no subtitles obtained yet; unsettled), Fetched (subtitle file and transcript recorded; settled) or Unavailable (settled without captions under the `late-video-data` policy; terminal). While Missing, a video SHALL also record when its subtitles were last checked and how many checks failed or were inconclusive. A new video SHALL start Missing, never checked, with no failed attempts. A video stored before this policy with a recorded first check without captions SHALL keep that time as its last check, with no failed attempts. A video reset for redownload SHALL return to Missing, with no subtitle filename, never checked and no failed attempts.

#### Scenario: New video
- **WHEN** a video is newly added to a tracked playlist or channel
- **THEN** its subtitle state is Missing, it has no recorded subtitle filename, and its subtitles were never checked

#### Scenario: Library upgraded
- **WHEN** the system starts on a database where a Missing video recorded its first check without captions at a given time
- **THEN** that video's subtitles are recorded as last checked at that time, with no failed attempts

#### Scenario: Video reset for redownload
- **WHEN** a video whose subtitle state is Fetched or Unavailable is reset for redownload
- **THEN** its subtitle state is Missing, it has no recorded subtitle filename, its subtitles were never checked, and it has no failed attempts

### Requirement: Subtitle Fetch Scheduled After Download
The system SHALL schedule a subtitle fetch to run immediately each time a video finishes downloading, without waiting for the next reconcile pass. It SHALL NOT schedule a second one for a video that already has a subtitle fetch pending or running. Reconcile passes keep scheduling fetches for videos whose subtitles are still Missing and due for a check under the `late-video-data` policy, through the video sync they run for every stored video (see `video-sync`).

#### Scenario: Video finishes downloading
- **WHEN** a video finishes downloading
- **THEN** a subtitle fetch task for that video is scheduled to run now

#### Scenario: Subtitle fetch already queued
- **WHEN** a video finishes downloading while a subtitle fetch for it is already pending or running
- **THEN** no second subtitle fetch is scheduled

### Requirement: Subtitle Fetch Outcome
A subtitle fetch SHALL set the video's subtitle state to Fetched when it writes the subtitle file and stores the transcript. A check is conclusive when the downloader successfully reports the video's caption tracks without reporting any warning that can affect them (it can silently withhold tracks, e.g. when YouTube requires a token it does not have, and only says so in a warning). Warnings that cannot affect caption tracks SHALL be ignored: the downloader failing to write its cache, and notices that some video formats were skipped. Every fetch that contacts the downloader SHALL record its time as the subtitles' last check. When a conclusive check finds no selectable track, the state SHALL become Unavailable when the video is old and stay Missing when it is young (see `late-video-data`). When the check is inconclusive or the downloader fails, the fetch SHALL count one failed attempt, and the state SHALL become Unavailable when the video is old and has at least 3 failed attempts; otherwise it SHALL stay Missing.

#### Scenario: Captions fetched
- **WHEN** a subtitle fetch selects a track and writes it
- **THEN** the video's subtitle state is Fetched

#### Scenario: Old video with no original-language captions
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago whose subtitles were last checked 2 days ago
- **THEN** the video's subtitle state is Unavailable and no subtitle file is written

#### Scenario: Old video checked for the first time
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago (e.g. of a playlist added today) that was never checked
- **THEN** its subtitle state is Unavailable at once and no subtitle file is written

#### Scenario: Recent video with no captions yet
- **WHEN** a conclusive check finds no selectable track for a video published 2 days ago
- **THEN** its subtitle state stays Missing and the check time is recorded, so a check at least 24 hours later tries again

#### Scenario: Downloader warns while checking
- **WHEN** the downloader reports the caption tracks with a warning that can affect them (e.g. `Some subtitles may be missing`), none is selectable, the video was published 30 days ago and has no failed attempts
- **THEN** its subtitle state stays Missing with 1 failed attempt, and the warning is logged

#### Scenario: Old video's third failed check
- **WHEN** the downloader fails while probing captions for a video published 30 days ago whose subtitles have 2 failed attempts
- **THEN** its subtitle state is Unavailable

#### Scenario: Downloader only warns about its cache or skipped formats
- **WHEN** the downloader reports the caption tracks with only a cache write failure warning and a `formats have been skipped` warning, and none is selectable
- **THEN** the check is conclusive: no failed attempt is counted and nothing is logged as inconclusive

#### Scenario: Downloader fails
- **WHEN** the downloader fails while probing or downloading captions (e.g. rate limited, network error) for a video published 2 days ago
- **THEN** the video's subtitle state stays Missing, one failed attempt is counted, and no subtitle file or transcript is recorded
