## ADDED Requirements

### Requirement: Channel Avatar Is Recorded
The system SHALL record, on the channel itself, the local filename of its downloaded avatar image, whenever one was successfully resolved and downloaded during channel creation. A channel whose avatar could not be resolved or downloaded SHALL have no recorded avatar filename.

#### Scenario: Channel created with an avatar
- **WHEN** channel creation resolves and downloads an avatar image for the channel
- **THEN** the channel's recorded avatar filename becomes the local filename of that image

#### Scenario: Channel created without an avatar
- **WHEN** channel creation does not obtain an avatar image, whether because none was available or because downloading it failed
- **THEN** the channel has no recorded avatar filename

## MODIFIED Requirements

### Requirement: List Channels
The system SHALL provide an HTTP endpoint that returns every currently stored channel.

#### Scenario: Channels exist
- **WHEN** one or more channels have been created
- **THEN** the system returns all of them, each with its handle, name, immutable YouTube channel ID, quality, video limit, path, creation timestamp, and avatar filename (when present)

#### Scenario: No channels exist
- **WHEN** no channels have been created
- **THEN** the system returns an empty list rather than an error
