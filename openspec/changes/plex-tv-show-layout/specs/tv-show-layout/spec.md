## Purpose

Lays every tracked channel and playlist out on disk as a TV show that Plex, Kodi and Jellyfin read natively: a show directory with its own metadata and poster, and every video as a date-numbered episode in a season folder.

## ADDED Requirements

### Requirement: Tracked Source Directory Is A Show
The system SHALL treat each tracked playlist's and channel's output directory (its configured storage path under the videos root) as one show. The show directory SHALL contain a `tvshow.nfo` describing the show and, for a channel with a stored avatar, a `poster.jpg` copy of that avatar. The show NFO SHALL carry the show's title (the playlist's or channel's name as shown in yarrtube), its studio (the same name), a `uniqueid` of type `youtube` holding the YouTube channel ID or playlist ID, and the status `Continuing`. The show files SHALL be written when the source is created and rewritten on every reconcile pass of that source, so a renamed source or a changed avatar propagates and a deleted show file reappears.

#### Scenario: Channel created
- **WHEN** a channel is created
- **THEN** its output directory contains a `tvshow.nfo` with the channel's name, its YouTube channel ID as `uniqueid`, and, when the channel has a stored avatar, a `poster.jpg` holding that avatar's bytes

#### Scenario: Playlist created
- **WHEN** a YouTube-linked playlist is created
- **THEN** its output directory contains a `tvshow.nfo` with the playlist's name and its YouTube playlist ID as `uniqueid`, and no poster

#### Scenario: Show file deleted by hand
- **WHEN** a show's `tvshow.nfo` or `poster.jpg` is missing when a reconcile pass of that source runs
- **THEN** the pass writes it again

#### Scenario: Show NFO is well-formed XML
- **WHEN** a source's name contains a character that is illegal unescaped in XML
- **THEN** the written `tvshow.nfo` escapes it and is well-formed

### Requirement: Episode Number From Publish Timestamp
The system SHALL assign every video an episode number derived from its YouTube publish timestamp (UTC): the season is the publish year, and the episode is the publish month and day (`MMDD`) followed by a two-digit same-day index. The index SHALL be the next free index among the videos of the same show already numbered on that publish day, starting at `01`, so that two videos of one show published on the same day get distinct numbers. The number SHALL be assigned once, the first time the system needs it for a video, and SHALL be recorded on the video and never recomputed, even if the video's title or position changes or its publish timestamp is reported differently later.

#### Scenario: First video of its day
- **WHEN** a video published on 15 March 2026 is numbered and no other video of its show has a number on that day
- **THEN** its season is `2026` and its episode is `031501`

#### Scenario: Second video of the same day
- **WHEN** a video of the same show is numbered after another video of that show already holds `031501`
- **THEN** its episode is `031502`

#### Scenario: Same video tracked by a channel and a playlist
- **WHEN** one YouTube video is tracked by both a channel and a playlist
- **THEN** each stored copy is numbered independently within its own show

#### Scenario: Number never changes
- **WHEN** a numbered video's title changes, or its playlist position changes, or a later metadata fetch reports a different publish time
- **THEN** its recorded season and episode stay the same and its files are not renamed

### Requirement: Publish Timestamp Source
The system SHALL take the publish timestamp used for numbering from the downloader's own video information at the moment it first fetches the video (thumbnail-ahead fetch or full download), falling back to the video's publish date without a time when no timestamp is reported. It MUST NOT depend on the YouTube Data API for numbering, so a metadata failure never prevents a video from being named.

#### Scenario: Timestamp reported
- **WHEN** the downloader reports a publish timestamp for a video being fetched
- **THEN** the video is numbered from that timestamp

#### Scenario: Only a date reported
- **WHEN** the downloader reports a publish date but no time
- **THEN** the video is numbered from that date, taking the next free same-day index

#### Scenario: Metadata unavailable
- **WHEN** the YouTube Data API cannot be reached while a video downloads
- **THEN** the video is still numbered and saved under its episode name

### Requirement: Episode Files Live In A Season Folder
The system SHALL store a video's file, its thumbnail and its episode NFO directly inside `Season <season>` under the show directory, all sharing the episode base name `S<season>E<episode> - <sanitized title>` and differing only by extension (`.mp4`, `.jpg`, `.nfo`). The season folder SHALL be created on demand and SHALL NOT contain any other kind of file written by the system. The video's recorded filename and recorded thumbnail filename SHALL be the paths of those files relative to the show directory (for example `Season 2026/S2026E031501 - Title.mp4`).

#### Scenario: Video downloaded
- **WHEN** a video numbered `2026`/`031501` titled `Title` finishes downloading
- **THEN** its show directory contains `Season 2026/S2026E031501 - Title.mp4`, `Season 2026/S2026E031501 - Title.jpg` and `Season 2026/S2026E031501 - Title.nfo`, and its recorded filename is `Season 2026/S2026E031501 - Title.mp4`

#### Scenario: Two episodes of different years
- **WHEN** a show has videos published in 2025 and 2026
- **THEN** they are stored in `Season 2025` and `Season 2026` respectively

### Requirement: Legacy Layouts Are Not Served As Shows
The system SHALL NOT write any new file in the previous per-video-folder layout. A video whose recorded filename still points into a per-video folder or to a flat file (a library the `layout-migration` subcommand has not processed) SHALL remain playable in the web UI, SHALL NOT be moved by reconcile, and SHALL be reported by the migration subcommand.

#### Scenario: Unmigrated video
- **WHEN** a reconcile pass finds a downloaded video whose recorded filename is not under a `Season <year>` folder
- **THEN** the pass leaves its files where they are and does not reset it for redownload
