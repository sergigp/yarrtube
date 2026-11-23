## MODIFIED Requirements

### Requirement: Step Status
Every per-video step SHALL have a status from one vocabulary: Pending (not done yet, no failed attempt), Retrying (failed 1 to 10 consecutive times, next attempt scheduled), Done (succeeded), Unavailable (YouTube doesn't have it; final, except a download Unavailable for a recheckable reason, see `video-download`) or Failed (gave up after 11 consecutive failed attempts). The download additionally has In Progress while it runs, and calls Done "Downloaded". A step SHALL record its consecutive failed attempts and when the last one failed. In other specs, a video "Excluded" is an Unavailable download, "Errored" a Retrying or Failed download, subtitles or summary "Missing" are Pending, subtitles "Fetched" and a summary "Generated" are Done, an `unavailable` verdict is a `failed` verdict, and "settled" late data is Done, Unavailable or Failed.

#### Scenario: New video
- **WHEN** a video is added to a tracked playlist
- **THEN** its download, thumbnail, subtitles, chapters, quality check, summary and categorization are all Pending

#### Scenario: Step that failed twice
- **WHEN** a video's subtitle check failed twice in a row
- **THEN** its subtitles are Retrying with 2 failed attempts

#### Scenario: Old video without captions
- **WHEN** a conclusive check finds no captions for a video published 30 days ago
- **THEN** its subtitles are Unavailable with no failed attempts
