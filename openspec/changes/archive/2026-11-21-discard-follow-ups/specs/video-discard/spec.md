# Spec Delta

## MODIFIED Requirements

### Requirement: A Discarded Video Stays Discarded After Leaving Its Source
When a reconcile removes a Discarded video from its source (evicted from a channel's window, or no longer in a YouTube-linked playlist), the source SHALL remember the video's YouTube id. A later reconcile of that source SHALL NOT store a remembered video again, and a channel SHALL count a remembered upload toward its limit (see `channel-video-sync`). Deleting the source, or restoring its discarded videos, SHALL forget its remembered videos.

#### Scenario: Channel window reaches back to an evicted discarded video
- **WHEN** a channel with `video_limit` 2 evicts its Discarded upload C, and a later reconcile runs after the limit is raised to 3
- **THEN** C is not stored again, no `VideoAddedToChannel` is published for it, and it takes the third slot, so the channel keeps 2 videos

#### Scenario: Discarded video re-added to a playlist
- **WHEN** a Discarded video leaves a YouTube-linked playlist, is removed by a reconcile, and is later added back to the playlist on YouTube
- **THEN** the next reconcile doesn't store it again and publishes no `VideoAddedToPlaylist` for it

#### Scenario: Source deleted and tracked again
- **WHEN** a channel with a remembered discarded video is deleted and then tracked again
- **THEN** the video is treated as any new upload

## ADDED Requirements

### Requirement: Restore A Source's Discarded Videos
The system SHALL let a client restore a channel's or playlist's discarded videos (`POST /api/channels/{handle}/discarded/restore`, `POST /api/playlists/{id}/discarded/restore`, answering 204). The source SHALL forget every video it remembers as discarded, each of its stored Discarded videos SHALL be back to Pending with no attempts, its thumbnail and subtitles to be fetched again, and one reconcile pass of the source SHALL run before the response. An unknown source SHALL answer 404.

#### Scenario: Restore a stored discarded video
- **WHEN** a client restores the discarded videos of a channel whose upload B, among its 3 most recent, is stored as Discarded
- **THEN** the request answers 204, B is Pending with no attempts, and its video sync is scheduled, so it is downloaded again

#### Scenario: Restore a discarded video that had subtitles
- **WHEN** a client restores the discarded videos of a playlist whose video B was downloaded with its thumbnail and subtitles before being discarded
- **THEN** B's thumbnail and subtitles are Pending, so they are fetched again along with its download

#### Scenario: Restore a remembered video back in the source
- **WHEN** a client restores the discarded videos of a playlist that remembers video C, which is in the playlist on YouTube again
- **THEN** C is no longer remembered, the reconcile stores it as Pending and publishes `VideoAddedToPlaylist` for it

#### Scenario: Remembered video no longer in the source
- **WHEN** a client restores the discarded videos of a channel with `video_limit` 2 that remembers upload D, which is no longer among its 2 most recent uploads
- **THEN** D is no longer remembered and is not stored

#### Scenario: Nothing discarded
- **WHEN** a client restores the discarded videos of a channel with no discarded video
- **THEN** the request answers 204 and only a reconcile pass of the channel runs

#### Scenario: Unknown source
- **WHEN** a client restores the discarded videos of a playlist id that isn't tracked
- **THEN** the request answers 404

#### Scenario: Listing fails during the restore
- **WHEN** a client restores the discarded videos of a channel and listing its uploads fails
- **THEN** the request answers 204, the channel's stored Discarded videos are Pending and its remembered videos are forgotten, and the next successful reconcile adds those still among its most recent uploads
