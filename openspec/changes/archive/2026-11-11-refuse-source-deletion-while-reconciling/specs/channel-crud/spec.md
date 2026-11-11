## MODIFIED Requirements

### Requirement: Delete Channel
The system SHALL provide an HTTP endpoint that deletes a previously created channel identified by its handle. Deleting a channel SHALL also remove its pending recurring reconcile, so no reconcile of the channel stays queued after it is gone. Deleting a channel SHALL also remove every pending task of the videos it deletes, so no work for a video that no longer exists stays queued; a task of those videos already running is left to finish. Deleting a channel while a sync of that same channel is running SHALL be refused, so a running sync can never write videos for a channel that no longer exists.

#### Scenario: Successful deletion
- **WHEN** a request identifies a channel handle that exists in storage
- **THEN** the system removes it and confirms the deletion

#### Scenario: Deleting a channel with a pending reconcile
- **WHEN** a request deletes a channel whose next recurring reconcile is pending
- **THEN** by the time the system confirms the deletion, no reconcile of that channel is pending, and pending tasks of other channels and playlists are unchanged

#### Scenario: Deleting a channel whose videos have pending tasks
- **WHEN** a request deletes a channel one of whose videos has a pending download waiting for its next attempt, and pending thumbnail and summary tasks
- **THEN** by the time the system confirms the deletion, no task of that video is pending, and pending tasks of other channels' and playlists' videos are unchanged

#### Scenario: Deleting a channel whose video has a running task
- **WHEN** a request deletes a channel while a task of one of its videos is running
- **THEN** the running task is not removed, and it finishes without error once it finds its video gone

#### Scenario: Channel added again after deletion
- **WHEN** a channel is deleted and then created again with the same handle
- **THEN** the channel has a single recurring reconcile chain, as if it had never been tracked before

#### Scenario: Deleting a nonexistent channel
- **WHEN** a request identifies a channel handle that does not exist in storage
- **THEN** the system reports that nothing was found, makes no change to storage, and returns a bad request with a meaningful error description

#### Scenario: Deleting a channel while it is being synced
- **WHEN** a request deletes a channel while a sync of that channel is running
- **THEN** the system refuses with a conflict and the message "channel <handle> is being synced, try again once it finishes", and makes no change to storage: the channel, its video records and its pending tasks remain, and no ChannelDeleted event is published

#### Scenario: Deleting a channel while another source is being synced
- **WHEN** a request deletes a channel while a sync of a different channel or playlist is running
- **THEN** the system deletes the channel as usual
