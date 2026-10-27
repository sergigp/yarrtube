# youtube-contract-tests Specification

## Purpose

Opt-in tests that check yarrtube's adapters against the real `yt-dlp` and YouTube Data API, so a change on YouTube's side is caught by a test, not in production, and so a fake YouTube has verified behaviour to reproduce.

## Requirements

### Requirement: Opt-In Contract Test Run
The YouTube contract tests SHALL be skipped by the default test run (and so by CI), and SHALL run together through one documented command that needs `yt-dlp`, `ffprobe`, network access and a `YOUTUBE_API_KEY`.

#### Scenario: Default test run
- **WHEN** a developer or CI runs the default test suite
- **THEN** no YouTube contract test runs and none is reported as failed

#### Scenario: Contract run
- **WHEN** a developer runs the contract test command with the prerequisites in place
- **THEN** every YouTube contract test runs against the real `yt-dlp` and YouTube

### Requirement: Stable Public Fixtures
The contract tests SHALL use only public videos, playlists and channels that are Creative-Commons licensed or maintained by the project, and that are unlikely to change, and SHALL name each one with its source in the tests.

#### Scenario: Fixture credited
- **WHEN** a contract test uses a YouTube video, playlist or channel
- **THEN** the tests state its id, what it is and its licence or owner

### Requirement: yt-dlp Download Contract
The contract tests SHALL verify that downloading a short video yields an `.mp4` file with H.264 video and AAC audio, named after the video's id, with a `.jpg` thumbnail next to it, its duration in whole seconds and its publish time, and that downloading a video YouTube does not have reports a failure carrying YouTube's reason.

#### Scenario: Playable download
- **WHEN** the contract tests download the short fixture video at low quality
- **THEN** the result is `<id>.mp4` (H.264 + AAC) with `<id>.jpg`, the video's duration and its publish time

#### Scenario: Missing video
- **WHEN** the contract tests download a video id YouTube does not have
- **THEN** the download is reported as failed with YouTube's unavailability message

### Requirement: yt-dlp Metadata Contract
The contract tests SHALL verify the `yt-dlp` calls that download nothing but metadata: the thumbnail-only fetch, the failure diagnosis, the caption probe with the SubRip subtitle fetch, and a channel's newest-videos listing.

#### Scenario: Thumbnail only
- **WHEN** the contract tests fetch the fixture video's thumbnail
- **THEN** `<id>.jpg` is written with the video's publish time, and no media file

#### Scenario: Diagnosis
- **WHEN** the contract tests diagnose a playable video and a missing one
- **THEN** the playable one has no failure reason and the missing one reports YouTube's reason

#### Scenario: Captions
- **WHEN** the contract tests probe a video with uploaded English and French subtitles and fetch the English ones
- **THEN** the probe lists `en` and `fr` as uploaded tracks, and the fetch writes `<id>.en.srt` holding SubRip cues

#### Scenario: Channel listing
- **WHEN** the contract tests list the fixture channel's two newest videos
- **THEN** they get those two videos' ids and titles, newest first

### Requirement: YouTube Data API Contract
The contract tests SHALL verify the YouTube Data API adapters: resolving a playlist and listing its videos, finding a video's metadata, resolving a channel handle, downloading the channel's avatar, and reporting unknown playlists, videos and handles as not found.

#### Scenario: Playlist
- **WHEN** the contract tests resolve the fixture playlist and list its videos
- **THEN** they get its title and video count, and its videos with titles in playlist order

#### Scenario: Video metadata
- **WHEN** the contract tests look up the fixture video
- **THEN** they get its title, description, channel, publish time, tags and category

#### Scenario: Channel and avatar
- **WHEN** the contract tests resolve the fixture channel's handle and store its avatar
- **THEN** they get its channel id, title and an avatar URL, and the avatar is stored as an image file

#### Scenario: Unknown ids
- **WHEN** the contract tests look up a playlist, video or handle that does not exist
- **THEN** each is reported as not found rather than as an error
