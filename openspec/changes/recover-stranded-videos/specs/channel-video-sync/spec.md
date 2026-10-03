## ADDED Requirements

### Requirement: Stranded Video Recovery (Channels)
The system SHALL, for every channel, during each reconcile pass trigger a download for every video of the channel that is stranded. A video is stranded when its download status is Pending, In Progress or Errored Retrying and it has no download task pending or running. The pass SHALL leave the video's download status unchanged, SHALL apply no cooldown, and SHALL NOT treat as stranded a video that the same pass newly adds or resets for redownload. A video whose status is Downloaded, Excluded or Errored is never stranded.

#### Scenario: Errored-retrying video with no download task
- **WHEN** a reconcile pass finds a video in status Errored Retrying with no download task pending or running for it
- **THEN** the system triggers a download for that video and leaves its status Errored Retrying

#### Scenario: In-progress video with no download task
- **WHEN** a reconcile pass finds a video in status In Progress with no download task pending or running for it
- **THEN** the system triggers a download for that video

#### Scenario: Non-terminal video whose download task is still queued
- **WHEN** a reconcile pass finds a video in status Pending, In Progress or Errored Retrying that has a download task pending or running
- **THEN** the system does not trigger another download for it

#### Scenario: Settled video is never stranded
- **WHEN** a reconcile pass finds a video in status Downloaded, Excluded or Errored with no download task
- **THEN** the stranded-video recovery does not trigger a download for it
