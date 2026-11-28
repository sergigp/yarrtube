# Spec Delta

## ADDED Requirements

### Requirement: Discarding A Video Unfavorites It
Discarding a favorite video SHALL unfavorite its YouTube video on every stored copy (see `video-favorites`), so a kept copy in another source is no longer protected from its next reconcile.

#### Scenario: Discard a favorite
- **WHEN** a client discards a favorite Downloaded video
- **THEN** the video is Discarded and is not a favorite

#### Scenario: Discard a copy of a favorite tracked twice
- **WHEN** a favorite YouTube video is tracked by a channel and a playlist, and a client discards the channel's copy
- **THEN** the playlist's copy is no longer a favorite either
