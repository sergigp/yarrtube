# video-write-consistency Specification

## Purpose

Guarantees that concurrent updates of one stored video, made by different operations (downloads, late-data fetches, reconcile passes, watch-state reports), never lose each other's effects.

## Requirements

### Requirement: Concurrent Updates Of A Video Are Never Lost
When two operations update the same stored video at overlapping times, the system SHALL keep the effect of both: each update SHALL be applied on top of the video as the other one left it, never on a copy read before the other stored its change. This SHALL hold for every operation that changes a stored video, including the synchronous watch-state reports from the web UI, which SHALL still answer within the same request.

#### Scenario: Watch progress reported while subtitles are fetched
- **WHEN** a video's playback progress is reported while that video's subtitle fetch records its subtitles
- **THEN** once both complete, the video has both the reported playback position and the fetched subtitles

#### Scenario: Watch progress reported while the quality is upgraded
- **WHEN** a video's playback progress is reported while that video's quality upgrade records the new height
- **THEN** once both complete, the video has both the reported playback position and the new height, and no further upgrade is attempted for the old height

#### Scenario: Marking watched while another copy finishes downloading
- **WHEN** a video is marked watched while another stored copy of the same YouTube video, in a different playlist or channel, finishes its download
- **THEN** once both complete, that copy is Downloaded with its file and episode number recorded, and is watched

#### Scenario: Download completes while a reconcile pass refreshes the title
- **WHEN** a video's download records its file between the time a reconcile pass reads that video and the time the pass stores its refreshed title
- **THEN** the video keeps the status and file set by the download, and gains the refreshed title

#### Scenario: Video removed while an update is in flight
- **WHEN** a stored video is removed from its source while another operation is updating it
- **THEN** the video stays removed: the update does not bring it back
