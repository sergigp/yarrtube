# smoke-tests Specification

## Purpose

Gives yarrtube an automated, end-to-end smoke suite that proves the real Docker image serves a working webapp — creating playlists/channels, downloading real videos via yt-dlp, and playing them back — runnable both in CI on every change and on demand before/after large refactors.

## Requirements

### Requirement: Dockerized Test Run
The system SHALL provide a script that builds the release Docker image from the repository's `Dockerfile`, runs a container from it with a fresh, isolated database and videos directory, and waits for `GET /status` to return `200` before running any test, failing with a non-zero exit code and the container's logs if readiness is not reached within a bounded timeout.

#### Scenario: Container becomes ready
- **WHEN** the script starts the built image
- **THEN** it polls `GET /status` until it returns `200`, then proceeds to run the test suite

#### Scenario: Container never becomes ready
- **WHEN** `GET /status` does not return `200` within the timeout
- **THEN** the script exits non-zero, prints the container's logs, and does not attempt to run the test suite

### Requirement: Guaranteed Cleanup
The system SHALL remove the container and any temporary database/videos directories it created after the run, whether the run succeeded, failed, or was interrupted.

#### Scenario: Test run fails
- **WHEN** the Playwright suite reports failures
- **THEN** the script still removes the running container and temporary directories before exiting non-zero

### Requirement: Failure Diagnostics
The system SHALL, when the run fails, preserve the container's logs and the Playwright run's traces/videos as files on disk before cleanup, so they can be inspected or uploaded as CI artifacts.

#### Scenario: A Playwright assertion fails
- **WHEN** a test in the suite fails
- **THEN** the failing test's trace and video are retained on disk, and the container's logs up to that point are saved to a file

### Requirement: Playlist Golden Path
The system SHALL verify, through the browser, that adding a `youtube_linked` playlist results in its video being downloaded and playable, and that deleting the playlist removes it from the UI.

#### Scenario: Add playlist, download, play, delete
- **WHEN** a user adds the maintainer-owned "yarrtube-smoke-tests" YouTube playlist through the Add dialog
- **THEN** the playlist appears in the sidebar, a download task appears in the Tasks view, the video's status progresses to Downloaded, the video appears in the playlist detail view and the home feed with a thumbnail and duration, and it plays back in the browser
- **WHEN** the user then deletes the playlist from the sidebar
- **THEN** it no longer appears in the sidebar or the home feed

### Requirement: Channel Golden Path
The system SHALL verify, through the browser, that adding a `youtube_linked` channel results in at least one video being downloaded and playable, without depending on that video's specific identity, and that deleting the channel removes it from the UI.

#### Scenario: Add channel, download, play, delete
- **WHEN** a user adds the `@BlenderOfficial` channel with a video limit of 1 through the Add dialog
- **THEN** the channel appears in the sidebar, at least one of its videos reaches Downloaded status, and that video plays back in the browser
- **WHEN** the user then deletes the channel from the sidebar
- **THEN** it no longer appears in the sidebar or the home feed

### Requirement: Sync Action Coverage
The system SHALL verify that triggering the sidebar "Sync" action on an already-tracked playlist or channel completes without error.

#### Scenario: Sync an existing playlist
- **WHEN** a user clicks the Sync button on a tracked playlist in the sidebar
- **THEN** the action completes without showing an error

### Requirement: Add-Dialog Error Handling Coverage
The system SHALL verify that the Add dialog surfaces a visible error message when creating a playlist with a path already used by another playlist, and when creating a channel with an invalid handle.

#### Scenario: Duplicate playlist path
- **WHEN** a user submits the Add dialog for a playlist whose path is already used by another tracked playlist
- **THEN** the dialog shows an error message and the Advanced options section is expanded

#### Scenario: Invalid channel handle
- **WHEN** a user submits the Add dialog for a channel with a handle that does not resolve to a real channel
- **THEN** the dialog shows an error message

### Requirement: Mobile Sidebar Coverage
The system SHALL verify that, at a narrow (mobile) viewport, the sidebar can be opened via the menu button and closed.

#### Scenario: Open and close the mobile sidebar
- **WHEN** a user viewing the app at a mobile viewport width taps the menu button
- **THEN** the sidebar becomes visible, and tapping its close control or the overlay hides it again

### Requirement: Continuous Integration
The system SHALL run the smoke suite automatically in CI on every pull request and on every push to `main`, using a `YOUTUBE_API_KEY` provided as a repository secret.

#### Scenario: Pull request opened
- **WHEN** a pull request is opened or updated
- **THEN** the smoke-tests workflow runs the dockerized suite and reports its result on the pull request
