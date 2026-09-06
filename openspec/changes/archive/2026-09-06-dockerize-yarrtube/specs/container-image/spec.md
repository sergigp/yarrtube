## Purpose

Defines how yarrtube is packaged and run as a Docker container so it can operate unattended on a NAS: its default process, the tasks that can be run against it ad hoc, and the volumes/ports it exposes to the host.

## ADDED Requirements

### Requirement: Container Image Build
The system SHALL be distributable as a single Docker image containing the yarrtube binary and all runtime dependencies needed to download videos (including a video/audio muxing tool), such that no additional software needs to be installed on the host to run any of its tasks.

#### Scenario: Building the image
- **WHEN** the Docker image is built from the repository
- **THEN** the resulting image can run the `download` and `update-ytdlp` tasks without any additional host-installed dependencies

### Requirement: Default Container Process
The system SHALL run the daemon process as the container's default entrypoint when the container is started with no command override, so that starting the container requires no additional configuration.

#### Scenario: Starting the container
- **WHEN** the container is started with no command override
- **THEN** the daemon process starts as the container's main process and the container keeps running

### Requirement: Ad Hoc Task Invocation
The system SHALL allow running the `download` and `update-ytdlp` tasks against an already-running container without restarting it or interrupting the daemon process.

#### Scenario: Running a task in a live container
- **WHEN** an operator runs `download <playlist_url> <output_path>` or `update-ytdlp` inside a running yarrtube container
- **THEN** the task runs to completion and exits with a status code reflecting success or failure, without stopping or restarting the container's daemon process

### Requirement: Video Output Volume
The system SHALL write downloaded videos to a configurable directory inside the container, such that mounting a host directory at that path makes the downloaded files accessible outside the container (for example, to a media server).

#### Scenario: Downloading with a mounted output directory
- **WHEN** the container is started with a host directory mounted at the video output path, and a playlist is downloaded into that path
- **THEN** the downloaded video files appear in the mounted host directory

### Requirement: Published Status Port
The system SHALL listen for HTTP connections on a configurable port that can be published to the host, so that the daemon's HTTP server is reachable from outside the container.

#### Scenario: Reaching the daemon from the host
- **WHEN** the container's HTTP port is published to the host
- **THEN** a request sent from outside the container to the published port reaches the daemon's HTTP server
