# Spec Delta

## MODIFIED Requirements

### Requirement: Subtitle Fetch Scheduled After Download
The system SHALL schedule a subtitle fetch to run immediately each time a video finishes downloading, without waiting for the next reconcile pass. It SHALL NOT schedule a second one for a video that already has a subtitle fetch pending or running. Reconcile passes keep scheduling fetches for videos whose subtitles are still Missing, through the video sync they run for every stored video (see `video-sync`).

#### Scenario: Video finishes downloading
- **WHEN** a video finishes downloading
- **THEN** a subtitle fetch task for that video is scheduled to run now

#### Scenario: Subtitle fetch already queued
- **WHEN** a video finishes downloading while a subtitle fetch for it is already pending or running
- **THEN** no second subtitle fetch is scheduled
