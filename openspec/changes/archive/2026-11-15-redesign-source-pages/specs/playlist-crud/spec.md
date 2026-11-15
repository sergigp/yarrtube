## ADDED Requirements

### Requirement: Listed Playlists Include Their Last Sync
Each playlist returned by the endpoint listing every playlist SHALL include when its videos were last synced from YouTube, as described in "Reconcile Records The Last Sync" (playlist-reconciliation), and SHALL report it as absent for a playlist not synced since it was tracked or since the upgrade that introduced it.

#### Scenario: Synced playlist
- **WHEN** a playlist was last reconciled, with its items listed on YouTube, at 2026-11-15T09:00:00Z
- **THEN** the list of playlists returns it with that last sync time

#### Scenario: Never synced playlist
- **WHEN** a playlist has not completed such a reconcile yet
- **THEN** the list of playlists returns it with its last sync time absent
