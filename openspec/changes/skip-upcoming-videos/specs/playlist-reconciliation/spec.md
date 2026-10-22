## ADDED Requirements

### Requirement: Not Yet Watchable Videos Are Not Playlist Members (YouTube-Linked Playlists)
The system SHALL treat an item of the source YouTube playlist that YouTube
reports as an upcoming or currently live broadcast as if it were not in the
playlist at all, even when its privacy status is public or unlisted. Once
YouTube no longer reports it as upcoming or live, the next reconcile SHALL
treat it like any other member.

#### Scenario: Playlist contains an upcoming premiere not yet stored
- **WHEN** a reconcile pass for a YouTube-linked playlist finds a public item that YouTube reports as an upcoming broadcast, with no stored record for that playlist
- **THEN** the system does not store it, does not fetch its thumbnail, and does not publish a `VideoAddedToPlaylist` event for it

#### Scenario: Playlist contains a video that is live now
- **WHEN** a reconcile pass for a YouTube-linked playlist finds an item that YouTube reports as a live broadcast
- **THEN** the system ignores it the same way as an upcoming item

#### Scenario: Stored video is reported as upcoming
- **WHEN** a reconcile pass for a YouTube-linked playlist finds that a stored video is now reported as an upcoming broadcast
- **THEN** the system deletes its stored record and publishes a `VideoRemovedFromPlaylist` event, the same as for a video no longer in the playlist

#### Scenario: Premiere becomes watchable
- **WHEN** a reconcile pass runs after a previously ignored premiere is no longer reported as upcoming or live
- **THEN** the system stores it with status PENDING, the same as any new public item

#### Scenario: Live-status lookup fails
- **WHEN** looking up whether a page's items are upcoming or live fails
- **THEN** the playlist's listing fails, as for any other listing failure
