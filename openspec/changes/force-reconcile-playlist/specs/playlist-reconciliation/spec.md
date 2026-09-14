## ADDED Requirements

### Requirement: On-Demand Reconciliation
The system SHALL provide an HTTP endpoint that runs one reconcile pass for a
playlist immediately, on request, using the same reconcile behavior as the
recurring and creation-triggered passes (YouTube membership diff for
YouTube-linked playlists, filesystem healing for every playlist, and
scheduling of the next recurring reconcile), regardless of the playlist's
kind.

#### Scenario: On-demand reconcile of a YouTube-linked playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `youtube_linked`
- **THEN** the system fetches its current members from the YouTube Data API,
  diffs them against stored videos, reconciles the filesystem, and schedules
  the next recurring reconcile

#### Scenario: On-demand reconcile of a custom playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  exists in storage with kind `custom`
- **THEN** the system makes no YouTube API request, reconciles the
  filesystem, and schedules the next recurring reconcile

#### Scenario: On-demand reconcile does not disturb the existing schedule
- **WHEN** an on-demand reconcile runs for a playlist that already has a
  future recurring reconcile scheduled
- **THEN** the system leaves that previously scheduled reconcile in place in
  addition to scheduling a new one from the on-demand pass, rather than
  cancelling or replacing it

#### Scenario: On-demand reconcile of a nonexistent playlist
- **WHEN** a client requests an on-demand reconcile for a playlist ID that
  does not exist in storage
- **THEN** the system makes no YouTube request, persists nothing, performs no
  filesystem sweep, schedules no reconcile, and responds without error, the
  same as the recurring reconcile task does for a deleted playlist
