## REMOVED Requirements

### Requirement: Sync On Playlist Creation
**Reason**: Superseded by `playlist-reconciliation`, which runs one reconcile
pass for every newly created playlist regardless of kind, not just a
YouTube-membership fetch.
**Migration**: See `playlist-reconciliation`'s "Reconcile On Playlist
Creation" requirement.

### Requirement: Recurring Sync
**Reason**: Superseded by `playlist-reconciliation`'s recurring reconcile
pass, which covers every playlist kind and also heals filesystem drift, not
just YouTube membership.
**Migration**: See `playlist-reconciliation`'s "Recurring Reconciliation"
requirement.

### Requirement: Configurable Sync Interval
**Reason**: Superseded by `playlist-reconciliation`'s reconcile interval
configuration (same default, renamed).
**Migration**: See `playlist-reconciliation`'s "Configurable Reconcile
Interval" requirement.

### Requirement: New Video Persistence
**Reason**: Superseded by `playlist-reconciliation`, which keeps this exact
behavior but scopes it explicitly to YouTube-linked playlists (a custom
playlist's membership never comes from a YouTube fetch).
**Migration**: See `playlist-reconciliation`'s "New Video Persistence
(YouTube-Linked Playlists)" requirement.

### Requirement: Removed Video Cleanup
**Reason**: Superseded by `playlist-reconciliation`, which keeps this exact
behavior but scopes it explicitly to YouTube-linked playlists.
**Migration**: See `playlist-reconciliation`'s "Removed Video Cleanup
(YouTube-Linked Playlists)" requirement.

### Requirement: Sync Skipped For a Deleted Playlist
**Reason**: Superseded by `playlist-reconciliation`'s equivalent no-op
behavior, which additionally covers skipping the new filesystem
reconciliation step.
**Migration**: See `playlist-reconciliation`'s "Reconcile Skipped For a
Deleted Playlist" requirement.
