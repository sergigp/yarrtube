## ADDED Requirements

### Requirement: On-Demand Reconciliation
The system SHALL provide an HTTP endpoint that runs one reconcile pass for a
playlist immediately, on request, using the same membership-diff and
filesystem-healing behavior as the recurring and creation-triggered passes
(YouTube membership diff for YouTube-linked playlists, filesystem healing
for every playlist), regardless of the playlist's kind, without scheduling
or otherwise affecting any recurring reconcile task.

#### Scenario: On-demand reconcile of a YouTube-linked playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `youtube_linked`
- **THEN** the system fetches its current members from the YouTube Data API,
  diffs them against stored videos, and reconciles the filesystem

#### Scenario: On-demand reconcile of a custom playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `custom`
- **THEN** the system makes no YouTube API request and reconciles the
  filesystem

#### Scenario: On-demand reconcile does not affect the recurring schedule
- **WHEN** an on-demand reconcile runs for a playlist, whether or not it
  already has a future reconcile task pending (e.g. one scheduled by
  creation or the last recurring pass)
- **THEN** the system does not create, cancel, or otherwise modify any
  reconcile task; whatever was pending before the on-demand reconcile ran
  remains pending, unchanged, afterward — repeating the on-demand reconcile
  any number of times never changes the number of pending reconcile tasks

#### Scenario: On-demand reconcile of a nonexistent playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  does not exist in storage
- **THEN** the system makes no YouTube request, persists nothing, and
  performs no filesystem sweep, and responds without error, the same as the
  recurring reconcile task does for a deleted playlist
