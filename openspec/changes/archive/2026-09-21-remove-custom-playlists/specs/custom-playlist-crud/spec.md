## REMOVED Requirements

### Requirement: Create Custom Playlist
**Reason**: Custom playlists (playlists with no YouTube playlist behind
them) were never wired into the web UI and have no real usage; removing
them simplifies the domain back to a single playlist shape.
**Migration**: None. No custom playlists exist in the running instance.
Callers that used `POST /custom-playlists` have no replacement endpoint.

### Requirement: Custom Playlist Creation Publishes a Domain Event
**Reason**: Only applied to custom playlists, which no longer exist as a
concept. Regular playlist creation continues to publish `PlaylistCreated`
per `playlist-crud`.
**Migration**: None.

### Requirement: Add Video To Custom Playlist
**Reason**: Only applied to custom playlists, which no longer exist as a
concept.
**Migration**: None. Callers that used `POST /custom-playlists/{id}/videos`
have no replacement endpoint.

### Requirement: Remove Video From Custom Playlist
**Reason**: Only applied to custom playlists, which no longer exist as a
concept.
**Migration**: None. Callers that used
`DELETE /custom-playlists/{id}/videos/{video_id}` have no replacement
endpoint.
