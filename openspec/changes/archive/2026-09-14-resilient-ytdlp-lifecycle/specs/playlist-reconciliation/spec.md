## ADDED Requirements

### Requirement: Permanently Failed Video Recovery
The system SHALL, for every playlist regardless of kind, also reset any video whose status is permanently errored back to PENDING and trigger a fresh download of it during each reconcile pass, with no limit on how many times a given video may be recovered this way.

#### Scenario: Permanently errored video found during reconcile
- **WHEN** a reconcile pass finds a video whose status is permanently errored
- **THEN** the system resets that video to PENDING, clears its recorded filename and quality, and triggers a fresh download of it

#### Scenario: Video errors again after being recovered
- **WHEN** a video that was previously reset by reconcile-driven recovery fails and permanently errors again
- **THEN** the next reconcile pass recovers it again, the same as any other permanently errored video
