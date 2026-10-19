## MODIFIED Requirements

### Requirement: Subtitle Fetch Outcome
A subtitle fetch SHALL set the video's subtitle state to Fetched when it writes the subtitle file and stores the transcript. A check is conclusive when the downloader successfully reports the video's caption tracks without reporting any warning that can affect them (it can silently withhold tracks, e.g. when YouTube requires a token it does not have, and only says so in a warning). Warnings that cannot affect caption tracks SHALL be ignored: the downloader failing to write its cache, and notices that some video formats were skipped. When a conclusive check finds no selectable track, the fetch SHALL record that time as the first check without captions if none is recorded yet, and SHALL set the state to Unavailable only when at least 7 days have passed both since the video was published (its record's creation time when no publish time is reported) and since that first check without captions. Otherwise the state SHALL stay Missing.

#### Scenario: Captions fetched
- **WHEN** a subtitle fetch selects a track and writes it
- **THEN** the video's subtitle state is Fetched

#### Scenario: Old video with no original-language captions
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago whose first check without captions was recorded 8 days ago
- **THEN** the video's subtitle state is Unavailable and no subtitle file is written

#### Scenario: Old video checked for the first time
- **WHEN** a conclusive check finds no selectable track for a video published 30 days ago (e.g. of a playlist added today) with no first check without captions recorded
- **THEN** its subtitle state stays Missing and now is recorded as its first check without captions, so later reconcile passes keep trying for 7 days

#### Scenario: Downloader warns while checking
- **WHEN** the downloader reports the caption tracks with a warning that can affect them (e.g. `Some subtitles may be missing`) and none is selectable
- **THEN** the video's subtitle state stays Missing, no first check without captions is recorded, and the warning is logged

#### Scenario: Downloader only warns about its cache or skipped formats
- **WHEN** the downloader reports the caption tracks with only a cache write failure warning and a `formats have been skipped` warning, and none is selectable
- **THEN** the check is conclusive: the first check without captions is recorded (or the state becomes Unavailable once the 7-day conditions hold) and nothing is logged as inconclusive

#### Scenario: Recent video with no captions yet
- **WHEN** a subtitle fetch finds no selectable track for a video published 2 days ago
- **THEN** the video's subtitle state stays Missing, so a later reconcile pass tries again

#### Scenario: Downloader fails
- **WHEN** the downloader fails while probing or downloading captions (e.g. rate limited, network error)
- **THEN** the video's subtitle state stays Missing and no subtitle file or transcript is recorded
