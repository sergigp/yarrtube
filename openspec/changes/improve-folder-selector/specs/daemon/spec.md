## ADDED Requirements

### Requirement: Startup Default Storage Directory Creation
The system SHALL, on startup, ensure the default parent storage directories (`playlists` and `channels`) exist under the configured videos root, creating any that are missing. A directory that already exists SHALL be left untouched, with its contents unchanged.

This SHALL happen at daemon startup rather than when the container image is built. The videos root is a mount point, and a host directory mounted there at container start replaces the path entirely, masking anything the image created under it — so directories created at build time would never be visible to a running container.

Failure to create a directory SHALL NOT prevent the daemon from starting: the daemon SHALL log the failure and continue, consistent with its other startup steps, so that a read-only or not-yet-ready mount does not take the service down.

#### Scenario: Default directories missing
- **WHEN** the daemon starts and the videos root contains neither default parent directory
- **THEN** the daemon creates both of them and logs that it did so

#### Scenario: Default directories already present
- **WHEN** the daemon starts and the default parent directories already exist under the videos root
- **THEN** the daemon leaves them and their contents unchanged

#### Scenario: Default directory creation fails
- **WHEN** the daemon starts and a default parent directory cannot be created, for example because the videos root is mounted read-only
- **THEN** the daemon logs the failure clearly, identifying it as a storage directory problem, and continues starting up rather than exiting

#### Scenario: Directories are visible to a browsing client
- **WHEN** the daemon has started against an empty mounted videos root
- **THEN** a client listing the videos root sees the default parent directories
