# Spec Delta

## ADDED Requirements

### Requirement: Listed Videos Include Their Favorite State
Every listed video (a channel's, a playlist's, a category's, home's, the title search's and the favorites list) SHALL report whether it is a favorite (`favorite`). A channel's or playlist's videos SHALL also report whether the video is kept (`kept`): a favorite no longer among the source's current videos that the source keeps (see `channel-video-sync` and `playlist-reconciliation`).

#### Scenario: Favorite reported in a channel list
- **WHEN** a client lists a channel's videos and one of them is a favorite
- **THEN** that video reports `favorite: true` and the others `favorite: false`

#### Scenario: Kept video reported
- **WHEN** a client lists a channel's videos and one favorite has aged out of the window
- **THEN** that video reports `favorite: true` and `kept: true`, and is listed after the channel's listed uploads

#### Scenario: Favorite reported on home
- **WHEN** a favorite downloaded video appears in home's latest videos
- **THEN** its card reports `favorite: true`
