## ADDED Requirements

### Requirement: SABR-Only Streaming Is Surfaced
The system SHALL detect, during a video download attempt, when `yt-dlp`
reports that YouTube's SABR-only streaming experiment is active for the
session — the condition under which the better formats are skipped because
they carry no downloadable URL. When that condition is detected, the system
SHALL emit a distinct warn-level event that an operator can recognize and
search for without reading surrounding output, carrying the affected video's
id and the reason text `yt-dlp` reported as structured fields (per the
`logging` capability's conventions). The system SHALL surface this condition
whether the download then failed outright or completed by falling back to a
lower-quality format, so that a merely degraded download is flagged and not
mistaken for a healthy one. When `yt-dlp` does not report the SABR-only
signal, the system SHALL NOT emit the event. Detecting and surfacing the
condition SHALL NOT, by itself, change which formats or clients the download
requests, the recorded failure reason, or the video's status and retry
behavior.

#### Scenario: Download fails under SABR-only streaming
- **WHEN** a video download attempt fails and `yt-dlp` reported the SABR-only
  streaming signal (the better formats were skipped as missing a URL)
- **THEN** the system emits a distinct warn-level SABR event with the video id
  and the reported reason as fields

#### Scenario: Download degrades to a lower-quality format under SABR-only streaming
- **WHEN** a video download attempt completes successfully but `yt-dlp`
  reported the SABR-only streaming signal, having skipped the better formats
  and fallen back to a lower-quality one
- **THEN** the system emits the same distinct warn-level SABR event with the
  video id and the reported reason as fields, in addition to recording the
  download as succeeded

#### Scenario: Download without the SABR-only signal
- **WHEN** a video download attempt runs and `yt-dlp` does not report the
  SABR-only streaming signal
- **THEN** the system does not emit the SABR event

#### Scenario: Surfacing does not alter download outcome
- **WHEN** the SABR-only streaming signal is detected during a download attempt
- **THEN** the video's recorded status, failure reason, and retry/exclusion
  behavior are exactly what they would have been had the signal only been
  logged, and no change is made to the formats or clients requested
