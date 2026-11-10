## MODIFIED Requirements

### Requirement: Errored Download Coverage
The suite SHALL verify that a video whose download fails is shown as Retrying with the reason the downloader reported and its next attempt, that its download is listed pending for that attempt with the reason, and that "Retry now" runs it again at once, after which the next attempt is further away.

#### Scenario: Download fails
- **WHEN** a user adds the fixture playlist holding a video whose download always fails
- **THEN** the video is shown as Retrying with the failure reason and "next attempt in 10 minutes", the Downloads tab lists its download pending with the reason, and after "Retry now" and another failure the video shows "next attempt in 1 hour"
