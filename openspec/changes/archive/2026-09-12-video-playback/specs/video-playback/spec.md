## Purpose

Serves the file content of downloaded videos over HTTP, directly from the same directory tree used for downloads, so they can be played back in a browser without any additional storage, indexing, or authentication step.

## ADDED Requirements

### Requirement: Serve Downloaded Video File Content
The system SHALL make a downloaded video's file content retrievable over HTTP, at a URL derived from its playlist's storage path and its recorded filename, without requiring authentication.

#### Scenario: Requesting a downloaded video's file
- **WHEN** a client requests the media URL of a video that has completed downloading
- **THEN** the daemon responds with HTTP status 200 and the exact bytes of that video's file, with a content type appropriate to the file

#### Scenario: Requesting a video that has no file yet
- **WHEN** a client requests a media URL for which no file currently exists on disk (not yet downloaded, removed, or an unrecognized path)
- **THEN** the daemon responds with HTTP status 404 and does not return file content

### Requirement: Media Served From The Configured Videos Root
The system SHALL serve video file content from within the same root directory used to store downloaded videos, so a video becomes retrievable as soon as its download completes, without a separate publishing or indexing step.

#### Scenario: Video finishes downloading
- **WHEN** a video's download completes and its file is saved under the configured videos root
- **THEN** that file is immediately retrievable over HTTP, with no further action

### Requirement: Range Requests Are Supported
The system SHALL support HTTP range requests when serving video file content, so a browser video player can seek within a video without downloading it from the start.

#### Scenario: Client requests a byte range
- **WHEN** a client requests a specific byte range of a video's file
- **THEN** the daemon responds with HTTP status 206 and only the requested range of bytes

### Requirement: Requests Cannot Escape The Videos Root
The system SHALL NOT serve any file located outside the configured videos root directory, regardless of the path requested.

#### Scenario: Request attempts to traverse outside the videos root
- **WHEN** a client requests a media URL containing a path segment that would resolve outside the configured videos root
- **THEN** the daemon does not return any file outside that root
