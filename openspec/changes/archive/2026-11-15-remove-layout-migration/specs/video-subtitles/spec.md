## MODIFIED Requirements

### Requirement: Subtitle Fetch Is Best Effort
A subtitle fetch SHALL run as its own scheduled task, separately from the video download. Its failure SHALL NOT fail the task and SHALL NOT change the video's download status or any download field; the fetch is re-queued under the `video-retries` policy instead. A subtitle fetch for a video that is not Downloaded, has no episode number, is still stored in the previous per-video-folder layout (it never gets subtitles), or whose subtitles are neither Pending nor Retrying SHALL make no changes and SHALL NOT contact the downloader. A fetch SHALL place nothing when, by the time its subtitles are downloaded, the video is no longer Downloaded with the same recorded filename.

#### Scenario: Fetch runs for a video already Fetched
- **WHEN** a subtitle fetch runs for a video whose subtitles are already Done, Unavailable or Failed
- **THEN** it makes no changes to the video or to disk

#### Scenario: Fetch runs for a video no longer downloaded
- **WHEN** a subtitle fetch runs for a video that has meanwhile been reset for redownload
- **THEN** it makes no changes and does not contact the downloader

#### Scenario: Fetch runs for a deleted video
- **WHEN** a subtitle fetch runs after its video's record was deleted
- **THEN** it completes without changes

#### Scenario: Video still in the previous layout
- **WHEN** a subtitle fetch runs for a Downloaded, numbered video whose recorded filename is still `My Video/My Video.mp4` or `My Video.mp4` (the previous layout)
- **THEN** it makes no changes, writes no file and does not contact the downloader, and reconcile passes do not schedule a subtitle fetch for it

#### Scenario: Video reset for redownload while its subtitles are fetched
- **WHEN** a video is reset for redownload after a subtitle fetch for it has started but before it places the subtitle file
- **THEN** no subtitle file is placed, no transcript is stored, no subtitles-fetched event is published, and its subtitles stay as they were

#### Scenario: Video deleted while its subtitles are fetched
- **WHEN** a video's record is deleted (or its playlist or channel deleted) after a subtitle fetch for it has started but before it places the subtitle file
- **THEN** no subtitle file is placed, no transcript is stored and no subtitles-fetched event is published

#### Scenario: Recording subtitles preserves download progress
- **WHEN** a video's download fields change between the time a subtitle fetch reads the video and the time it records the subtitles
- **THEN** the video keeps its new download fields and gains the recorded subtitle status and filename

