# Spec Delta

## ADDED Requirements

### Requirement: Stop Retrying A Video
The system SHALL let a client stop retrying a video (`POST /api/videos/{id}/stop-retrying`, answering 204). Every Retrying step of the video except the categorization SHALL become Failed, keeping its failed attempts, and its pending task SHALL be removed. Other steps, and the categorization, SHALL be unchanged. A video with nothing Retrying SHALL change nothing. An unknown video SHALL answer 404. "Retry A Video" undoes it.

#### Scenario: Stop retrying a failing download
- **WHEN** a client stops retrying a video whose download is Retrying with 1 failed attempt and its next attempt due in 5 minutes
- **THEN** its download is Failed with 1 failed attempt, and no download task is pending for it

#### Scenario: Other steps untouched
- **WHEN** a client stops retrying a video whose download is Retrying and whose thumbnail is Pending with a fetch task queued
- **THEN** the download is Failed, the thumbnail is still Pending and its fetch task is still queued

#### Scenario: Categorization left alone
- **WHEN** a client stops retrying a video whose download and categorization are both Retrying
- **THEN** the download is Failed, and the categorization is still Retrying with its task still queued

#### Scenario: Nothing Retrying
- **WHEN** a client stops retrying a video whose steps are all Done or Pending
- **THEN** the server answers 204 and nothing changes

#### Scenario: Unknown video
- **WHEN** a client stops retrying a video id that does not exist
- **THEN** the server answers 404

#### Scenario: Retried afterwards
- **WHEN** a client retries a video whose download was stopped and is Failed
- **THEN** the download is Pending with no failed attempts and is scheduled to run now
