# Spec Delta

## MODIFIED Requirements

### Requirement: Removed Video Cleanup (YouTube-Linked Playlists)
The system SHALL delete a stored video's record when a reconcile pass for a
YouTube-linked playlist finds that it is no longer present in the source
YouTube playlist, unless it is a favorite, which is kept instead (see
"Favorites Are Kept (YouTube-Linked Playlists)"). It SHALL remember a removed
Discarded video's YouTube id for
the playlist before deleting its record, SHALL log each such deletion, and
SHALL publish a
`VideoRemovedFromPlaylist` event carrying that video's title, its recorded
filename (if any), and whether it had been downloaded.

#### Scenario: Previously stored video no longer in the YouTube playlist
- **WHEN** a reconcile pass finds that a video previously stored for a YouTube-linked playlist, not a favorite, is no longer a member of that playlist on YouTube
- **THEN** the system deletes its stored record and logs the deletion

#### Scenario: Deletion publishes a VideoDeleted event
- **WHEN** a reconcile pass deletes a stored video's record because it is no longer in the source YouTube playlist
- **THEN** the system publishes a `VideoRemovedFromPlaylist` event containing the playlist ID, the video ID, the video's title, its recorded filename (if any), and whether the video had been downloaded

#### Scenario: Removing a discarded video remembers it
- **WHEN** a reconcile pass deletes the record of a video whose download is Discarded because it is no longer in the source YouTube playlist
- **THEN** the playlist remembers its YouTube id, then its stored record is deleted

## ADDED Requirements

### Requirement: Favorites Are Kept (YouTube-Linked Playlists)
A reconcile pass SHALL keep a stored favorite that is no longer in the source YouTube playlist instead of removing it: its record, episode number, files and pending work stay, no `VideoRemovedFromPlaylist` is published, and it is marked kept. A video favorited while the pass runs SHALL be kept too.

#### Scenario: Favorite removed from the YouTube playlist
- **WHEN** a stored favorite numbered `S01E004` is no longer in its YouTube playlist
- **THEN** it stays stored as `S01E004` with its files, is marked kept, and no `VideoRemovedFromPlaylist` is published for it

#### Scenario: Kept video's number is not reused
- **WHEN** the kept favorite holds the playlist's highest episode `6` and a later pass stores a new video
- **THEN** the new video's episode is `7`

#### Scenario: Favorited during the pass
- **WHEN** a stored video is favorited while the listing that no longer has it runs
- **THEN** it is kept, not removed

#### Scenario: Unfavorited kept video
- **WHEN** a kept video of a playlist is unfavorited and the next reconcile pass doesn't find it in the YouTube playlist
- **THEN** it is removed like any video no longer in the playlist, and its files are deleted

### Requirement: Kept Videos Sit After The Listed Ones (YouTube-Linked Playlists)
A reconcile pass SHALL place the playlist's kept videos after its listed ones, in the order they had before. A kept video that is back in the YouTube playlist SHALL stop being kept and take its playlist position.

#### Scenario: Kept video placed last
- **WHEN** a reconcile pass lists items A, B, C at positions 0–2 and keeps the favorite D
- **THEN** the playlist's videos are ordered A, B, C, D

#### Scenario: Kept video re-added on YouTube
- **WHEN** a kept video is added back to its YouTube playlist at position 1
- **THEN** it is no longer kept and is listed at position 1
