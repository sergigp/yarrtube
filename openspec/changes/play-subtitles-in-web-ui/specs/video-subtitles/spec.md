## ADDED Requirements

### Requirement: Subtitles Served As WebVTT
The system SHALL provide HTTP endpoints that return a video's fetched subtitles in WebVTT format, addressed by the video's owning playlist or channel and the video's YouTube ID: `GET /api/playlists/{id}/videos/{video_id}/subtitles.vtt` and `GET /api/channels/{handle}/videos/{video_id}/subtitles.vtt`. The body SHALL be produced from the video's recorded subtitle file when requested, keeping every cue's timing and text, and SHALL be served with content type `text/vtt; charset=utf-8`. Serving SHALL NOT write any file to the videos root. The system SHALL respond with HTTP status 404 when the playlist or channel is not tracked, when the video is not recorded for it, when the video's subtitles are not fetched, or when its recorded subtitle file no longer exists, and with HTTP status 400 for an invalid playlist ID, channel handle or video ID.

#### Scenario: Requesting a playlist video's subtitles
- **WHEN** a client requests the subtitles of a playlist video whose subtitle file contains the cue `00:00:01,500 --> 00:00:03,000` with text `Hello there`
- **THEN** the daemon responds with HTTP status 200, content type `text/vtt; charset=utf-8`, and a body that starts with `WEBVTT` and contains the cue `00:00:01.500 --> 00:00:03.000` with text `Hello there`

#### Scenario: Requesting a channel video's subtitles
- **WHEN** a client requests the subtitles of a channel video whose subtitles are fetched
- **THEN** the daemon responds with HTTP status 200 and the video's subtitles in WebVTT format

#### Scenario: Video without fetched subtitles
- **WHEN** a client requests the subtitles of a video whose subtitle state is Missing or Unavailable
- **THEN** the daemon responds with HTTP status 404

#### Scenario: Subtitle file missing from disk
- **WHEN** a client requests the subtitles of a video whose subtitles are Fetched but whose subtitle file no longer exists
- **THEN** the daemon responds with HTTP status 404

#### Scenario: Video not recorded for the source
- **WHEN** a client requests the subtitles of a video ID under a playlist or channel that does not record that video
- **THEN** the daemon responds with HTTP status 404

#### Scenario: Unknown source
- **WHEN** a client requests subtitles under a playlist ID or channel handle that is not tracked
- **THEN** the daemon responds with HTTP status 404

#### Scenario: Nothing written to the library
- **WHEN** a client requests a video's subtitles
- **THEN** the video's season folder still contains only the files it held before the request
