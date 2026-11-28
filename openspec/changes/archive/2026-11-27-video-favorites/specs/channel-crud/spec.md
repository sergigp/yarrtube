# Spec Delta

## ADDED Requirements

### Requirement: Listed Channels Include Their Favorite Count
Each listed channel SHALL include its count of favorite videos (`favorite_count`), Unavailable and Discarded ones not counted, so a client can tell what deleting it would delete.

#### Scenario: Channel with favorites
- **WHEN** a channel has two favorite videos, one of them kept, and three that aren't
- **THEN** it is listed with `favorite_count` 2

#### Scenario: Channel without favorites
- **WHEN** a channel has no favorite video
- **THEN** it is listed with `favorite_count` 0
