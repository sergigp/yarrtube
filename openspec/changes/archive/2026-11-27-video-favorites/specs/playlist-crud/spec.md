# Spec Delta

## ADDED Requirements

### Requirement: Listed Playlists Include Their Favorite Count
Each listed playlist SHALL include its count of favorite videos (`favorite_count`), Unavailable and Discarded ones not counted, so a client can tell what deleting it would delete.

#### Scenario: Playlist with favorites
- **WHEN** a playlist has one favorite video and four that aren't
- **THEN** it is listed with `favorite_count` 1

#### Scenario: Playlist without favorites
- **WHEN** a playlist has no favorite video
- **THEN** it is listed with `favorite_count` 0
