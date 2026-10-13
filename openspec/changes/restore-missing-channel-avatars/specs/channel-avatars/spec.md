## ADDED Requirements

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
