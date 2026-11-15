## ADDED Requirements

### Requirement: Reconcile Records The Last Sync
At the end of every reconcile pass of a playlist (initial, recurring or on demand) whose listing of the playlist's items on YouTube succeeded, the system SHALL record the current time as the playlist's last sync time. A pass whose listing failed SHALL leave the recorded time as it was. Recording it SHALL NOT undo a change to the playlist's settings saved while the pass ran, and saving the playlist's settings SHALL NOT undo a recorded last sync time.

#### Scenario: Successful pass
- **WHEN** a recurring reconcile pass of a playlist lists its items on YouTube and finishes at 2026-11-15T09:00:00Z
- **THEN** the playlist's last sync time is 2026-11-15T09:00:00Z

#### Scenario: Listing fails
- **WHEN** a reconcile pass of a playlist last synced at 2026-11-14T09:00:00Z fails to list its items
- **THEN** its last sync time stays 2026-11-14T09:00:00Z

#### Scenario: Settings saved meanwhile
- **WHEN** a playlist's reconcile interval is changed while a reconcile pass of it runs
- **THEN** after the pass the playlist has both the new interval and the new last sync time
