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

### Requirement: Channel Avatar Copied As Show Poster
The system SHALL copy a channel's stored avatar into the channel's show directory as `poster.jpg` whenever it writes the channel's show files (on creation and on every reconcile pass, see `tv-show-layout`), overwriting any existing poster so a changed avatar propagates. A channel with no stored avatar SHALL get no poster, and an existing `poster.jpg` SHALL then be left as it is.

#### Scenario: Channel with a stored avatar
- **WHEN** a channel with a recorded avatar filename has its show files written
- **THEN** its show directory contains a `poster.jpg` with the avatar's bytes

#### Scenario: Channel without a stored avatar
- **WHEN** a channel with no recorded avatar filename has its show files written
- **THEN** no `poster.jpg` is written and an existing one is left untouched

### Requirement: Missing Channel Avatar Restored On Reconcile
The system SHALL, at the start of every channel reconcile pass and before writing the channel's show files, check whether the channel's avatar is stored. When the channel has no recorded avatar filename, or its recorded avatar file is missing from local storage, the system SHALL look the channel up on YouTube again, download its avatar image, store it, and record the resulting local filename on the channel. The show files written in the same pass SHALL use the restored avatar. A failure to look up, download, store or record the avatar SHALL NOT fail the reconcile pass, and the next pass SHALL try again. A channel whose recorded avatar file is present SHALL NOT have its avatar downloaded again.

#### Scenario: Recorded avatar file is missing
- **WHEN** a channel reconcile pass runs for a channel whose recorded avatar file is missing from local storage and YouTube returns an avatar image URL for it
- **THEN** the system downloads and stores that image, records its filename on the channel, and copies it into the channel's show directory as `poster.jpg`

#### Scenario: Channel has no recorded avatar
- **WHEN** a channel reconcile pass runs for a channel with no recorded avatar filename and YouTube returns an avatar image URL for it
- **THEN** the system downloads and stores that image and records its filename on the channel

#### Scenario: Recorded avatar file is present
- **WHEN** a channel reconcile pass runs for a channel whose recorded avatar file is present in local storage
- **THEN** the system does not download the avatar again

#### Scenario: Restoring the avatar fails
- **WHEN** a channel reconcile pass runs for a channel whose avatar is missing and looking it up or downloading it fails
- **THEN** the channel's recorded avatar filename is left unchanged and the rest of the pass still syncs the channel's videos
