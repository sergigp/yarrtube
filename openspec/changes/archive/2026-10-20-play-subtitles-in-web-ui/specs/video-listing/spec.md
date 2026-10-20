## ADDED Requirements

### Requirement: Listed Videos Include Their Subtitle Language
The system SHALL include each video's subtitle language in the responses that list the videos of a playlist and of a channel. The language SHALL be the code its subtitle file is named with (the base code, e.g. `es` for an `es-US` track). The system SHALL report it as absent for a video whose subtitles are not fetched.

#### Scenario: Video with fetched subtitles is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video whose subtitles were fetched as `Season 2026/S2026E031501 - Title.en.srt`
- **THEN** that video's entry reports its subtitle language as `en`

#### Scenario: Video without fetched subtitles is listed
- **WHEN** a client lists the videos of a tracked playlist or channel that has a video whose subtitle state is Missing or Unavailable
- **THEN** that video's entry reports its subtitle language as absent, and every other field is still returned
