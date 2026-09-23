## ADDED Requirements

### Requirement: Download Failure Reason Is Recorded
The system SHALL record `yt-dlp`'s actual reported error text as a failed download attempt's failure reason, when `yt-dlp` reported one, instead of a generic message that does not distinguish one failure cause from another.

#### Scenario: yt-dlp reports a specific error
- **WHEN** a video download attempt fails and `yt-dlp` reported an error message on its standard error stream
- **THEN** that exact message is recorded as the attempt's failure reason

#### Scenario: yt-dlp fails without reporting a message
- **WHEN** a video download attempt fails and `yt-dlp` reported no error message
- **THEN** a generic failure reason is recorded instead
