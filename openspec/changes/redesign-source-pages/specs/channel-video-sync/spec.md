## ADDED Requirements

### Requirement: Reconcile Records The Last Sync
At the end of every reconcile pass of a channel (initial, recurring or on demand) whose listing of the channel's videos succeeded, the system SHALL record the current time as the channel's last sync time. A pass whose listing failed SHALL leave the recorded time as it was. Recording it SHALL NOT undo a change to the channel's settings saved while the pass ran, and saving the channel's settings SHALL NOT undo a recorded last sync time.

#### Scenario: Successful pass
- **WHEN** a recurring reconcile pass of a channel lists its videos and finishes at 2026-11-15T09:00:00Z
- **THEN** the channel's last sync time is 2026-11-15T09:00:00Z

#### Scenario: Listing fails
- **WHEN** a reconcile pass of a channel last synced at 2026-11-14T09:00:00Z fails to list its videos
- **THEN** its last sync time stays 2026-11-14T09:00:00Z

#### Scenario: Settings saved meanwhile
- **WHEN** a channel's video limit is changed while a reconcile pass of it runs
- **THEN** after the pass the channel has both the new video limit and the new last sync time
