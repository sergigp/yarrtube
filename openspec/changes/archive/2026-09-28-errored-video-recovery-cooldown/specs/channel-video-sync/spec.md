## MODIFIED Requirements

### Requirement: Permanently Failed Video Recovery (Channels)
The system SHALL, for every channel, during each reconcile pass reset any
video that has been permanently errored for at least 24 hours back to
PENDING and trigger a fresh download of it. A video that became
permanently errored less than 24 hours before the pass SHALL be left
permanently errored and SHALL NOT have a download triggered by that pass.
There is no limit on how many times a given video may be recovered this way.

#### Scenario: Permanently errored video found during reconcile
- **WHEN** a reconcile pass finds a video that has been permanently errored for 24 hours or more
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Video permanently errored less than 24 hours ago
- **WHEN** a reconcile pass finds a video that became permanently errored less than 24 hours earlier
- **THEN** the system leaves the video permanently errored and does not trigger a download for it
