## ADDED Requirements

### Requirement: Initial Reconcile After Playlist Creation
The system SHALL run one reconcile pass for a playlist as soon as practical
after it is created.

#### Scenario: Newly created YouTube-linked playlist gets reconciled
- **WHEN** a YouTube-linked playlist is successfully created
- **THEN** the system fetches its current members from the YouTube Data API and persists them

### Requirement: On-Demand Playlist Reconciliation
The system SHALL provide an HTTP endpoint that runs one reconcile pass for a
playlist immediately, on request, using the same membership-diff and
filesystem-healing behavior as the recurring and creation-triggered passes,
without scheduling or otherwise affecting any recurring reconcile task.

#### Scenario: On-demand reconcile of a YouTube-linked playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `youtube_linked`
- **THEN** the system fetches its current members from the YouTube Data API,
  diffs them against stored videos, and reconciles the filesystem

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

## REMOVED Requirements

### Requirement: Reconcile On Playlist Creation
**Reason**: Restated without its custom-playlist scenario as "Initial Reconcile After Playlist Creation"; custom playlists were removed (#41).
**Migration**: None.

### Requirement: On-Demand Reconciliation
**Reason**: Restated without its custom-playlist scenario as "On-Demand Playlist Reconciliation"; custom playlists were removed (#41).
**Migration**: None.

### Requirement: YouTube Membership Diff Applies Only To YouTube-Linked Playlists
**Reason**: Custom playlists were removed (#41); every playlist is YouTube-linked, so the membership diff always applies.
**Migration**: None. The diff itself is still specified by the YouTube-linked membership requirements.
