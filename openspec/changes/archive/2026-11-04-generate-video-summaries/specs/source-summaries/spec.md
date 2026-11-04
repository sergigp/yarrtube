# Spec Delta

## MODIFIED Requirements

### Requirement: Source Summaries Setting
Every stored channel and playlist SHALL have a `summaries_enabled` boolean, `false` unless set. Sources stored before this setting existed SHALL have `false`. Only the videos of a source with it `true` get a summary (see `video-summaries`).

#### Scenario: Existing sources after upgrade
- **WHEN** the daemon starts on a database with channels and playlists stored before this setting existed
- **THEN** every one of them has `summaries_enabled` `false`
