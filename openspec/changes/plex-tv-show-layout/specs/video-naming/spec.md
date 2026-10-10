## MODIFIED Requirements

### Requirement: Video ID Omitted By Default
The system SHALL NOT include the video's YouTube ID in the episode base name derived for it. The episode base name SHALL be `S<season>E<episode> - <sanitized title>`, where the season and episode come from the video's recorded episode number (see `tv-show-layout`) and the sanitized title follows the rules above.

#### Scenario: No naming collision
- **WHEN** a video is downloaded
- **THEN** its file, thumbnail and NFO are named from its episode number and sanitized title alone, without the video ID

#### Scenario: Episode base name
- **WHEN** a video numbered `2026`/`031501` and titled `Some: Title?` is named
- **THEN** its episode base name is `S2026E031501 - Some- Title-`, without the video ID

## REMOVED Requirements

### Requirement: Collision Fallback Appends Video ID
**Reason**: The episode number is unique within a show (the same-day index guarantees it), so two videos can no longer derive the same name and no collision fallback is needed.
**Migration**: None; existing folders carrying a `[<video id>]` suffix are renamed by `migrate-layout` like any other.
