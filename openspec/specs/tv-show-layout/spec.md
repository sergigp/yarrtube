# tv-show-layout Specification

## Purpose

Lays every tracked channel and playlist out on disk as a TV show that Plex, Kodi and Jellyfin read natively: a show directory with its own metadata and poster, and every video as an episode in a season folder: a channel's numbered by publish date, a playlist's numbered in playlist order.

## Requirements

### Requirement: Tracked Source Directory Is A Show
The system SHALL treat each tracked playlist's and channel's output directory (its configured storage path under the videos root) as one show. The show directory SHALL contain a `tvshow.nfo` describing the show and, for a channel with a stored avatar, a `poster.jpg` copy of that avatar. The show NFO SHALL carry the show's title (the playlist's or channel's name as shown in yarrtube), its studio (the same name), a `uniqueid` of type `youtube` holding the YouTube channel ID or playlist ID, and the status `Continuing`. The show files SHALL be written when the source is created and written again on every reconcile pass of that source whenever missing or different (an unchanged file is left untouched), so a renamed source or a changed avatar propagates and a deleted show file reappears. A failure to write them SHALL NOT stop the pass; the next pass tries again.

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

### Requirement: Channel Episode Number From Publish Timestamp
The system SHALL assign every video of a channel an episode number derived from its YouTube publish timestamp (UTC): the season is the publish year, and the episode is the publish month and day (`MMDD`) followed by a two-digit same-day index. The index SHALL be the next free index among the videos of the same channel already numbered on that publish day, starting at `01`, so that two videos of one channel published on the same day get distinct numbers. The number SHALL be assigned once, the first time the system needs it for a video, and SHALL be recorded on the video and never recomputed, even if the video's title changes or its publish timestamp is reported differently later. A day holds at most 99 numbered videos of one channel: numbering a 100th SHALL fail rather than spill into the next day's numbers. Only a channel's videos SHALL be numbered by publish timestamp; a video in no channel without a recorded number SHALL fail to be numbered.

#### Scenario: First video of its day
- **WHEN** a channel video published on 15 March 2026 is numbered and no other video of its channel has a number on that day
- **THEN** its season is `2026` and its episode is `031501`

#### Scenario: Second video of the same day
- **WHEN** a video of the same channel is numbered after another video of that channel already holds `031501`
- **THEN** its episode is `031502`

#### Scenario: Number never changes
- **WHEN** a numbered channel video's title changes, or a later metadata fetch reports a different publish time
- **THEN** its recorded season and episode stay the same and its files are not renamed

### Requirement: Playlist Episode Number From Playlist Order
The system SHALL number every video of a playlist in playlist order, independently of its publish time: the season is always `1`, and the episode is the playlist's next episode number, one past the highest number the playlist has ever assigned (starting at `1`). The number SHALL be assigned and recorded when the playlist reconcile first stores the video, the reconcile storing a pass's new videos in their playlist position order, so the first pass of a playlist numbers it exactly in its YouTube order and videos added later are numbered after every existing one, wherever YouTube places them. A playlist's numbers SHALL never be reused, even after the video holding the highest one leaves the playlist or the playlist is edited while a pass stores new videos (the counter never decreases), and SHALL never change when the playlist is reordered or a video's title changes. A video that is never downloaded leaves its number unused.

#### Scenario: First pass of a playlist
- **WHEN** the first reconcile pass of a playlist stores its three videos at positions 0, 1 and 2
- **THEN** their numbers are season `1` episodes `1`, `2` and `3` in that order, whatever their publish times

#### Scenario: Video added to a numbered playlist
- **WHEN** a reconcile pass stores a new video at position 0 of a playlist whose highest assigned episode is `5`
- **THEN** the new video's episode is `6`

#### Scenario: Highest numbered video removed
- **WHEN** the video holding a playlist's highest episode `6` leaves the playlist and a later pass stores a new video
- **THEN** the new video's episode is `7`

#### Scenario: Playlist reordered
- **WHEN** a reconcile pass sees a numbered video at a different position than before
- **THEN** its recorded season and episode stay the same and its files are not renamed

#### Scenario: Playlist tracked before the upgrade
- **WHEN** the system starts on a database whose playlists hold videos recorded before episode numbers existed
- **THEN** every such video is numbered season `1` in its playlist's stored position order (ties by the order they were stored), and the next video stored in that playlist is numbered after them

### Requirement: Same Video In Several Shows
The system SHALL number each stored copy of a YouTube video within its own show, by that show's rule.

#### Scenario: Same video tracked by a channel and a playlist
- **WHEN** one YouTube video is tracked by both a channel and a playlist
- **THEN** the channel's copy is numbered from its publish timestamp and the playlist's copy in playlist order, independently

### Requirement: Publish Timestamp Source
The system SHALL take the publish timestamp used for numbering a channel video from the downloader's own video information at the moment it first fetches the video (thumbnail-ahead fetch or full download), falling back to the video's publish date without a time when no timestamp is reported. It MUST NOT depend on the YouTube Data API for numbering, so a metadata failure never prevents a video from being named.

#### Scenario: Timestamp reported
- **WHEN** the downloader reports a publish timestamp for a channel video being fetched
- **THEN** the video is numbered from that timestamp

#### Scenario: Only a date reported
- **WHEN** the downloader reports a publish date but no time for a channel video
- **THEN** the video is numbered from that date, taking the next free same-day index

#### Scenario: Metadata unavailable
- **WHEN** the YouTube Data API cannot be reached while a video downloads
- **THEN** the video is still numbered and saved under its episode name

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

### Requirement: Legacy Layouts Are Not Served As Shows
The system SHALL NOT write any new file in the previous per-video-folder layout. A video whose recorded filename still points into a per-video folder or to a flat file (a library the `layout-migration` subcommand has not processed) SHALL remain playable in the web UI, SHALL NOT be moved by reconcile, and SHALL be reported by the migration subcommand.

#### Scenario: Unmigrated video
- **WHEN** a reconcile pass finds a downloaded video whose recorded filename is not under a `Season <season>` folder
- **THEN** the pass leaves its files where they are and does not reset it for redownload

#### Scenario: Legacy folder titled like a season
- **WHEN** a legacy video's per-video folder is named after a title starting with `Season ` (e.g. `Season 2 Finale`)
- **THEN** it is treated as legacy: the migration subcommand reports and moves it, and reconcile never sweeps its files
