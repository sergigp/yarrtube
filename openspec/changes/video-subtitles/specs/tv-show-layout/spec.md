# Spec Delta

## MODIFIED Requirements

### Requirement: Episode Files Live In A Season Folder
The system SHALL store a video's file, its thumbnail, its episode NFO and its subtitle file directly inside `Season <season>` under the show directory, all sharing the episode base name `S<season>E<episode> - <sanitized title>` (a playlist episode's season padded to two digits and its episode to at least three, e.g. `S01E007`; a channel episode's episode padded to six, e.g. `S2026E031501`) and differing only by extension (`.mp4`, `.jpg`, `.nfo`, and `.<lang>.srt` for the subtitle file, `<lang>` being the base language code, e.g. `.en.srt`, or `.es.srt` for an `es-US` track). A subtitle file written after the download SHALL take the base name of the video's recorded filename, which is not renamed when the video's title later changes. The season folder SHALL be created on demand and SHALL NOT contain any other kind of file written by the system. The video's recorded filename, recorded thumbnail filename and recorded subtitle filename SHALL be the paths of those files relative to the show directory (for example `Season 2026/S2026E031501 - Title.mp4`).

#### Scenario: Video downloaded
- **WHEN** a video numbered `2026`/`031501` titled `Title` finishes downloading
- **THEN** its show directory contains `Season 2026/S2026E031501 - Title.mp4`, `Season 2026/S2026E031501 - Title.jpg` and `Season 2026/S2026E031501 - Title.nfo`, and its recorded filename is `Season 2026/S2026E031501 - Title.mp4`

#### Scenario: Playlist video downloaded
- **WHEN** a playlist video numbered season `1` episode `7` titled `Title` finishes downloading
- **THEN** its show directory contains `Season 1/S01E007 - Title.mp4`, `.jpg` and `.nfo`, and its recorded filename is `Season 1/S01E007 - Title.mp4`

#### Scenario: Subtitles fetched
- **WHEN** `en` subtitles are fetched for a playlist video numbered season `1` episode `7` titled `Title`
- **THEN** its show directory contains `Season 1/S01E007 - Title.en.srt`, and its recorded subtitle filename is `Season 1/S01E007 - Title.en.srt`

#### Scenario: Subtitles fetched after the video was retitled
- **WHEN** `en` subtitles are fetched for a video downloaded as `Season 1/S01E007 - Old.mp4` whose title has since changed to `New`
- **THEN** its show directory contains `Season 1/S01E007 - Old.en.srt`, next to `Season 1/S01E007 - Old.mp4`

#### Scenario: Playlist past 999 episodes
- **WHEN** a playlist video is numbered episode `1000`
- **THEN** its base name starts with `S01E1000`

#### Scenario: Two channel episodes of different years
- **WHEN** a channel has videos published in 2025 and 2026
- **THEN** they are stored in `Season 2025` and `Season 2026` respectively

#### Scenario: Playlist videos of different years
- **WHEN** a playlist holds videos published in 2014 and 2026
- **THEN** they are all stored in `Season 1`
