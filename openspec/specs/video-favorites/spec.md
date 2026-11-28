# video-favorites Specification

## Purpose
Lets the user mark the YouTube videos they care about as favorites, so they are never evicted from their channel or playlist and can be found together in one place.

## Requirements

### Requirement: Favorite A Video
The system SHALL let a client favorite a stored video (`PUT /api/videos/{id}/favorite`, answering 204), recording when it was favorited. Favoriting a video that is already a favorite SHALL answer 204 and change nothing, keeping its original favorite time.

#### Scenario: Favorite a downloaded video
- **WHEN** a client favorites a Downloaded video that is not a favorite
- **THEN** the request answers 204 and the video is a favorite, favorited now

#### Scenario: Favorite a video not downloaded yet
- **WHEN** a client favorites a video whose download is Pending
- **THEN** the request answers 204 and the video is a favorite, its download still Pending

#### Scenario: Favorite twice
- **WHEN** a client favorites a video favorited yesterday
- **THEN** the request answers 204 and the video is still favorited yesterday

### Requirement: Unfavorite A Video
The system SHALL let a client unfavorite a stored video (`DELETE /api/videos/{id}/favorite`, answering 204). Unfavoriting a video that is not a favorite SHALL answer 204 and change nothing.

#### Scenario: Unfavorite a favorite
- **WHEN** a client unfavorites a favorite video
- **THEN** the request answers 204 and the video is no longer a favorite

#### Scenario: Unfavorite a video that isn't a favorite
- **WHEN** a client unfavorites a video that is not a favorite
- **THEN** the request answers 204 and the video is unchanged

### Requirement: Favorite Belongs To The YouTube Video
Favoriting or unfavoriting a video SHALL apply to every stored copy of its YouTube video, whichever channel or playlist tracks it. A copy stored later, by any source, of a YouTube video that has a favorite copy SHALL start as a favorite with that copy's favorite time.

#### Scenario: Favorite applies to every copy
- **WHEN** a YouTube video is tracked by a channel and a playlist, and a client favorites the channel's copy
- **THEN** both copies are favorites, with the same favorite time

#### Scenario: Unfavorite applies to every copy
- **WHEN** a YouTube video is tracked by a channel and a playlist as a favorite, and a client unfavorites the playlist's copy
- **THEN** neither copy is a favorite

#### Scenario: New copy inherits the favorite
- **WHEN** a channel's copy of a YouTube video is a favorite and a playlist reconcile stores the same YouTube video
- **THEN** the playlist's new copy is a favorite with the channel copy's favorite time

### Requirement: Only Stored, Undiscarded Videos Can Be Favorited
Favoriting a video whose download is Discarded SHALL answer 409 and change nothing. Favoriting or unfavoriting an unknown video SHALL answer 404.

#### Scenario: Favorite a discarded video
- **WHEN** a client favorites a video whose download is Discarded
- **THEN** the request answers 409 and the video is not a favorite

#### Scenario: Unknown video
- **WHEN** a client favorites or unfavorites a video id that doesn't exist
- **THEN** the request answers 404

### Requirement: List Favorite Videos
The system SHALL list the favorite videos (`GET /api/favorites/videos`), once per YouTube video, with the same fields as a category's videos (its source and the categories having it included), most recently favorited first. Unavailable and Discarded videos SHALL be omitted. A YouTube video tracked by a channel and a playlist SHALL be listed from its channel.

#### Scenario: Most recently favorited first
- **WHEN** video A was favorited yesterday and video B today
- **THEN** the list is B, then A

#### Scenario: One entry per YouTube video
- **WHEN** a favorite YouTube video is tracked by a channel and a playlist
- **THEN** it is listed once, with the channel as its source

#### Scenario: Favorites not downloaded yet are listed
- **WHEN** a favorite video's download is Retrying
- **THEN** it is listed with its Retrying status

#### Scenario: No favorites
- **WHEN** no video is a favorite
- **THEN** the request answers 200 with an empty list
