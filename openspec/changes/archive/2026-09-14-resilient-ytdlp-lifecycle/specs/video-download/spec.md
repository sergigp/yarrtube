## MODIFIED Requirements

### Requirement: Automatic Retry On Download Failure
The system SHALL automatically retry a failed video download a bounded number of times before giving up on that attempt sequence, without any manual action. Exhausting that bounded sequence SHALL NOT prevent the video from being attempted again later by playlist reconciliation.

#### Scenario: Transient download failure
- **WHEN** a video download attempt fails and retries remain
- **THEN** the system automatically attempts the download again after a delay

#### Scenario: Retries exhausted
- **WHEN** a video download has failed on every allowed attempt
- **THEN** the system stops attempting that download within that attempt sequence, though the video may be attempted again later by playlist reconciliation
