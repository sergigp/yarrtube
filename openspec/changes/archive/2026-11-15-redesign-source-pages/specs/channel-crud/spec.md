## ADDED Requirements

### Requirement: Listed Channels Include Their Last Sync
Each channel returned by the endpoint listing every channel SHALL include when its videos were last synced from YouTube, as described in "Reconcile Records The Last Sync" (channel-video-sync), and SHALL report it as absent for a channel not synced since it was tracked or since the upgrade that introduced it.

#### Scenario: Synced channel
- **WHEN** a channel was last reconciled, with its videos listed on YouTube, at 2026-11-15T09:00:00Z
- **THEN** the list of channels returns it with that last sync time

#### Scenario: Never synced channel
- **WHEN** a channel has not completed such a reconcile yet
- **THEN** the list of channels returns it with its last sync time absent
