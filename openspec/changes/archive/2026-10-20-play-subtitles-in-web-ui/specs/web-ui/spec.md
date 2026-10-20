## ADDED Requirements

### Requirement: Player Offers Subtitles
When the selected video in a playlist or channel detail view is downloaded and has a subtitle language, the video player SHALL offer its subtitles as a captions track, loaded from that video's subtitles endpoint and labelled with the language's name in English (e.g. "English" for `en`, "Spanish" for `es`), falling back to the language code when no name is known. The track SHALL be off when playback starts. The user SHALL turn it on and off through the player's native captions control. When the selected video has no subtitle language, the player SHALL offer no captions track.

#### Scenario: Video with subtitles
- **WHEN** a user selects a downloaded video whose subtitle language is `en`
- **THEN** the player offers an "English" captions track, and no captions are shown until the user turns them on

#### Scenario: Turning captions on
- **WHEN** a user turns on the "English" captions track through the player's captions control
- **THEN** the video's subtitles are shown in sync with playback

#### Scenario: Video without subtitles
- **WHEN** a user selects a downloaded video with no subtitle language
- **THEN** the player offers no captions track

#### Scenario: Unknown language code
- **WHEN** a user selects a downloaded video whose subtitle language code has no known name
- **THEN** the captions track is labelled with the language code
