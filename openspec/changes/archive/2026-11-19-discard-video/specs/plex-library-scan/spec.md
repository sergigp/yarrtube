# Spec Delta

## ADDED Requirements

### Requirement: Show Folder Scanned After A Video's Files Are Deleted
When the integration is enabled, the system SHALL ask Plex to scan a video's show directory after deleting that video's files, whether the video was discarded or evicted from its channel or playlist, the same way and with the same retries as after a download. Deleting a video with no recorded file SHALL NOT request a scan.

#### Scenario: Discarded episode leaves Plex
- **WHEN** a Downloaded video of the channel stored at `channels/Some Channel` is discarded and its files are deleted
- **THEN** Plex is asked to scan `/volume1/media/yarrtube/channels/Some Channel` in the section containing it

#### Scenario: Evicted episode leaves Plex
- **WHEN** a Downloaded video is evicted from its playlist and its files are deleted
- **THEN** Plex is asked to scan the playlist's show folder

#### Scenario: Nothing deleted
- **WHEN** a video with no recorded file is evicted from its channel
- **THEN** no scan is requested
