## ADDED Requirements

### Requirement: Playlist Listing Failure Is Not Fatal
The system SHALL treat a failure to list a playlist's current items on
YouTube as non-fatal for the reconcile pass: it SHALL leave every stored
video of that playlist, and its membership, unchanged for that pass, SHALL
publish no membership event, and SHALL log the failure. The rest of the pass
SHALL still run from the stored videos, and a recurring pass SHALL still
schedule the next reconcile of the playlist.

#### Scenario: Listing fails during a recurring reconcile
- **WHEN** a recurring reconcile pass's attempt to list a playlist's current items fails (e.g. the playlist was deleted or made private on YouTube, or the YouTube Data API errors)
- **THEN** the system leaves that playlist's stored videos unchanged, publishes no `VideoAddedToPlaylist` or `VideoRemovedFromPlaylist` event, logs the failure, and does not treat the pass as failed

#### Scenario: Filesystem steps still run when listing fails
- **WHEN** a reconcile pass's attempt to list a playlist's current items fails and a stored video of that playlist needs healing (e.g. its downloaded file is missing)
- **THEN** the system still heals it the same way as in a pass whose listing succeeded

#### Scenario: Next reconcile still scheduled when listing fails
- **WHEN** a recurring reconcile pass's attempt to list a playlist's current items fails
- **THEN** the system still schedules the next reconcile of that playlist after the configured interval

#### Scenario: On-demand reconcile when listing fails
- **WHEN** an on-demand reconcile of an existing playlist runs and its attempt to list the playlist's current items fails
- **THEN** the system leaves that playlist's stored videos unchanged and responds without error
