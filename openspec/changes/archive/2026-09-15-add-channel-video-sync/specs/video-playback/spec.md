## MODIFIED Requirements

### Requirement: Serve Downloaded Video File Content
The system SHALL make a downloaded video's file content retrievable over HTTP, at a URL derived from its owning playlist's or channel's storage path and its recorded filename, without requiring authentication.

#### Scenario: Requesting a downloaded video's file
- **WHEN** a client requests the media URL of a video that has completed downloading
- **THEN** the daemon responds with HTTP status 200 and the exact bytes of that video's file, with a content type appropriate to the file

#### Scenario: Requesting a video that has no file yet
- **WHEN** a client requests a media URL for which no file currently exists on disk (not yet downloaded, removed, or an unrecognized path)
- **THEN** the daemon responds with HTTP status 404 and does not return file content
