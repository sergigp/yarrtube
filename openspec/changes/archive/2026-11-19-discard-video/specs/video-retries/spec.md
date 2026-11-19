# Spec Delta

## MODIFIED Requirements

### Requirement: Retry A Video
The system SHALL let a client retry a video (`POST /api/videos/{id}/retry`, answering 204). Retrying SHALL make every Retrying or Failed step of the video Pending with no failed attempts, delete the video's `failed` verdicts, and leave Pending, In Progress, Done and Unavailable steps, a Discarded download (see `video-discard`) and any data already obtained unchanged. The system SHALL then sync the video, so each step it needs is scheduled to run now, and a step whose retry was queued for later is moved to now. Retrying a video with nothing Retrying or Failed SHALL change nothing but still sync it. Retrying an unknown video SHALL answer 404.

#### Scenario: Retry a failed download
- **WHEN** a client retries a video whose download is Failed with 11 failed attempts
- **THEN** its download is Pending with no failed attempts and a download of it is scheduled to run now

#### Scenario: Retry a downloaded video whose AI failed
- **WHEN** a client retries a Downloaded video whose subtitles are Done, whose summary is Failed and which has a `failed` verdict for "Tech"
- **THEN** its file and subtitles are unchanged, its summary is Pending, "Tech" is pending again, and a summary is scheduled

#### Scenario: Retry a step waiting for its next attempt
- **WHEN** a client retries a video whose download is Retrying with its next attempt due in 6 hours
- **THEN** its download is Pending with no failed attempts and its queued download runs now

#### Scenario: Unavailable steps are kept
- **WHEN** a client retries a video whose subtitles are Unavailable and whose chapters are Failed
- **THEN** its subtitles stay Unavailable and its chapters are Pending

#### Scenario: Unknown video
- **WHEN** a client retries a video id that doesn't exist
- **THEN** the request answers 404

#### Scenario: Discarded download is kept
- **WHEN** a client retries a video whose download is Discarded and whose thumbnail is Failed
- **THEN** its download stays Discarded, its thumbnail is Pending, and no download of it is scheduled
