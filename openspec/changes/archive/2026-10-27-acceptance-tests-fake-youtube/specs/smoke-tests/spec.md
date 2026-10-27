## ADDED Requirements

### Requirement: Fake YouTube
The suite SHALL run the daemon against a fake YouTube: a fake YouTube Data API and a stand-in `yt-dlp` serving the same fixture catalogue of playlists, a channel and videos from files in the repository, so the suite needs no YouTube API key and makes no request to YouTube. Only the upgrade check's seeding phase, which runs the released image, SHALL use the real YouTube, and it SHALL be skipped when no YouTube API key is available.

#### Scenario: Suite without YouTube
- **WHEN** the suite runs with no YouTube API key and no access to YouTube
- **THEN** every test except the skipped upgrade check runs, and downloads come from the fixture catalogue

#### Scenario: Fixture video downloaded
- **WHEN** a test adds a fixture playlist
- **THEN** its video downloads within seconds, with its thumbnail and subtitles, and plays

### Requirement: Errored Download Coverage
The suite SHALL verify that a video whose download keeps failing has its download listed with the reason the downloader reported while it is retried, and is shown as errored once its retries run out.

#### Scenario: Download fails
- **WHEN** a user adds the fixture playlist holding a video whose download always fails
- **THEN** the Downloads tab lists its download with the failure reason while it is retried, and once the retries run out the video is shown as Errored

### Requirement: Search Coverage
The suite SHALL verify the sidebar's search and the Tasks view's search, which only appear once there are more than 15 sources or tasks.

#### Scenario: Sidebar search
- **WHEN** more than 15 playlists are tracked and a user types part of one's name into the sidebar search
- **THEN** the sidebar lists only the matching playlists

#### Scenario: Tasks search
- **WHEN** a user types part of a playlist's name into the Tasks view's search
- **THEN** only that playlist's tasks are listed

### Requirement: Premiere Coverage
The suite SHALL verify that a playlist's upcoming premiere is not listed or downloaded until it is watchable, while its other videos are.

#### Scenario: Upcoming premiere
- **WHEN** a user adds the fixture playlist holding an upcoming premiere and a regular video
- **THEN** only the regular video is listed and downloaded

### Requirement: Media Delivery Coverage
The suite SHALL verify that the media endpoint answers a byte-range request with a partial response, and that a downloaded video plays in the iPhone/WebKit browser.

#### Scenario: Range request
- **WHEN** the suite requests the video's media URL with `Range: bytes=0-1023`
- **THEN** the response is `206` with a `Content-Range` header and 1024 bytes

## MODIFIED Requirements

### Requirement: Playlist Golden Path
The system SHALL verify, through the browser, that adding a `youtube_linked` playlist results in its video being downloaded and playable, and that deleting the playlist removes it from the UI.

#### Scenario: Add playlist, download, play, delete
- **WHEN** a user adds the fake YouTube's fixture playlist through the Add dialog
- **THEN** the playlist appears in the sidebar, a download task appears in the Tasks view, the video's status progresses to Downloaded, the video appears in the playlist detail view and the home feed with a thumbnail and duration, and it plays back in the browser
- **WHEN** the user then deletes the playlist from the sidebar
- **THEN** it no longer appears in the sidebar or the home feed

### Requirement: Channel Golden Path
The system SHALL verify, through the browser, that adding a `youtube_linked` channel results in its newest videos being downloaded and playable, and that deleting the channel removes it from the UI.

#### Scenario: Add channel, download, play, delete
- **WHEN** a user adds the fake YouTube's fixture channel with a video limit of 1 through the Add dialog
- **THEN** the channel appears in the sidebar, its newest video reaches Downloaded status, and that video plays back in the browser
- **WHEN** the user then deletes the channel from the sidebar
- **THEN** it no longer appears in the sidebar or the home feed

## REMOVED Requirements

### Requirement: Media Compatibility Coverage
**Reason**: Against a fake YouTube the downloaded file is the repository's own fixture, so checking its codecs no longer says anything about real downloads.
**Migration**: The YouTube contract tests check that real downloads are H.264/AAC `.mp4`; the range request and WebKit playback checks continue as "Media Delivery Coverage".
