## ADDED Requirements

### Requirement: Listed Videos Include Their Steps
The system SHALL include, for each video in the responses that list the videos of a playlist, a channel or a category, its download status and a `steps` object with an entry for each of `download`, `thumbnail`, `subtitles`, `chapters`, `quality`, `summary` and `categorization`. Each entry SHALL hold the step's status (`PENDING`, `RETRYING`, `DONE`, `UNAVAILABLE` or `FAILED`; the download also `IN_PROGRESS`), its consecutive failed attempts, when its next attempt is due (only while Retrying) and its last failure reason (only while Retrying or Failed, and only where one is recorded). The download status SHALL be one of `PENDING`, `IN_PROGRESS`, `DOWNLOADED`, `RETRYING`, `FAILED` and `UNAVAILABLE`; `ERRORED_RETRYING`, `ERRORED` and `EXCLUDED` SHALL no longer be returned.

#### Scenario: Retrying video is listed
- **WHEN** a client lists the videos of a playlist with a video whose first download attempt failed at 10:00 with "HTTP Error 503"
- **THEN** that video's status is `RETRYING` and its `download` step is `RETRYING` with 1 failed attempt, next attempt at 10:10 and failure "HTTP Error 503"

#### Scenario: Failed video is listed
- **WHEN** a client lists a video whose download gave up after 11 attempts
- **THEN** that video's status is `FAILED` and its `download` step is `FAILED` with 11 failed attempts, its last failure reason and no next attempt

#### Scenario: Downloaded video with a failed summary
- **WHEN** a client lists a downloaded video whose subtitles are Done and whose summary gave up
- **THEN** its status is `DOWNLOADED`, its `download` and `subtitles` steps are `DONE` and its `summary` step is `FAILED`
