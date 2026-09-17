## Purpose

Resolves, downloads, and stores a visual avatar image for each tracked channel, and serves it back over HTTP, so the SPA can show a channel's own picture instead of only its name.

## Requirements

### Requirement: Channel Avatar Resolved At Creation
The system SHALL, when creating a channel, attempt to resolve an avatar image URL for it from the same YouTube Data API response already used to resolve its title, download that image, and store it in local storage, recording the resulting local filename on the channel. A failure to resolve or download the avatar SHALL NOT fail channel creation.

#### Scenario: Channel has an avatar available
- **WHEN** a channel is created and YouTube returns an avatar image URL for it
- **THEN** the system downloads and stores that image and records its local filename on the created channel

#### Scenario: Channel has no avatar available
- **WHEN** a channel is created and YouTube returns no avatar image URL for it
- **THEN** the channel is created successfully with no recorded avatar filename

#### Scenario: Avatar download fails
- **WHEN** a channel is created and YouTube returns an avatar image URL but downloading that image fails
- **THEN** the channel is still created successfully with no recorded avatar filename

### Requirement: Channel Avatar Served Over HTTP
The system SHALL serve a stored channel avatar image over HTTP at a location derived from its recorded filename, without requiring authentication.

#### Scenario: Requesting a stored avatar
- **WHEN** a client requests a channel's recorded avatar filename at its serving location
- **THEN** the daemon responds with HTTP status 200 and the image's bytes

#### Scenario: Requesting an avatar that was never stored
- **WHEN** a client requests an avatar filename that does not exist in local storage
- **THEN** the daemon responds with HTTP status 404

### Requirement: Channel Avatar Deleted With Its Channel
The system SHALL delete a channel's locally stored avatar file, if one was recorded, whenever that channel is deleted.

#### Scenario: Deleting a channel with a stored avatar
- **WHEN** a channel that has a recorded avatar filename is deleted
- **THEN** its stored avatar file is removed from local storage

#### Scenario: Deleting a channel with no stored avatar
- **WHEN** a channel that has no recorded avatar filename is deleted
- **THEN** no avatar file removal is attempted
