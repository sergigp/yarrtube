## MODIFIED Requirements

### Requirement: Container Image Build
The system SHALL be distributable as a single Docker image containing the yarrtube binary, a working `yt-dlp` binary, and all other runtime dependencies needed to download videos (including a video/audio muxing tool), such that no additional software needs to be installed on the host to run any of its tasks and a freshly built or freshly started container never begins without a usable `yt-dlp` binary already present.

#### Scenario: Building the image
- **WHEN** the Docker image is built from the repository
- **THEN** the resulting image can run the `download` and `update-ytdlp` tasks without any additional host-installed dependencies, and already contains a usable `yt-dlp` binary before any runtime update ever runs

#### Scenario: Container started without network access to fetch an update
- **WHEN** the container starts and the runtime `yt-dlp` self-update cannot reach the network
- **THEN** the `download` task still succeeds using the `yt-dlp` binary bundled into the image
