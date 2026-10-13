## MODIFIED Requirements

### Requirement: Channel Avatar Is Recorded
The system SHALL record, on the channel itself, the local filename of its downloaded avatar image, whenever one was successfully resolved and downloaded during channel creation or restored by a channel reconcile pass (see `channel-avatars`). A channel whose avatar could not be resolved or downloaded SHALL have no recorded avatar filename, unless one was recorded earlier.

#### Scenario: Channel created with an avatar
- **WHEN** channel creation resolves and downloads an avatar image for the channel
- **THEN** the channel's recorded avatar filename becomes the local filename of that image

#### Scenario: Channel created without an avatar
- **WHEN** channel creation does not obtain an avatar image, whether because none was available or because downloading it failed
- **THEN** the channel has no recorded avatar filename

#### Scenario: Avatar restored by a reconcile pass
- **WHEN** a channel reconcile pass restores a missing avatar and stores it under a filename different from the recorded one
- **THEN** the channel's recorded avatar filename becomes the new local filename
