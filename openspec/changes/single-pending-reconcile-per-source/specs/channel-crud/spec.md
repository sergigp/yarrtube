## MODIFIED Requirements

### Requirement: Delete Channel
The system SHALL provide an HTTP endpoint that deletes a previously created channel identified by its handle. Deleting a channel SHALL also remove its pending recurring reconcile, so no reconcile of the channel stays queued after it is gone.

#### Scenario: Successful deletion
- **WHEN** a request identifies a channel handle that exists in storage
- **THEN** the system removes it and confirms the deletion

#### Scenario: Deleting a channel with a pending reconcile
- **WHEN** a request deletes a channel whose next recurring reconcile is pending
- **THEN** by the time the system confirms the deletion, no reconcile of that channel is pending, and pending tasks of other channels and playlists are unchanged

#### Scenario: Channel added again after deletion
- **WHEN** a channel is deleted and then created again with the same handle
- **THEN** the channel has a single recurring reconcile chain, as if it had never been tracked before

#### Scenario: Deleting a nonexistent channel
- **WHEN** a request identifies a channel handle that does not exist in storage
- **THEN** the system reports that nothing was found, makes no change to storage, and returns a bad request with a meaningful error description
