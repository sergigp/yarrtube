# running-version Specification

## Purpose

Identifies which build of yarrtube is running, so an operator can tell at a glance whether an automatically released image has been picked up.

## Requirements

### Requirement: Release Builds Carry Their Release Tag
A container image built for a release SHALL report the release tag it was built from (e.g. `v0.9.2`) as its version, independently of the version declared in the package manifest.

#### Scenario: Image built for an automatically tagged release
- **WHEN** CI tags a merge to `main` as `v0.9.2` while the package manifest still declares `0.9.0`, and builds the release image
- **THEN** a daemon running that image reports version `v0.9.2`

#### Scenario: Image built for a manually pushed tag
- **WHEN** a `v1.0.0` tag is pushed by hand and its release image is built
- **THEN** a daemon running that image reports version `v1.0.0`

### Requirement: Edge Builds Carry Their Commit
A container image built by a manual (`:edge`) release run SHALL report `edge-<short commit sha>` as its version.

#### Scenario: Manual edge build
- **WHEN** the release workflow is run manually on commit `abc1234`
- **THEN** a daemon running that image reports version `edge-abc1234`

### Requirement: Unversioned Builds Report Dev
A binary built without a release version (e.g. a local build) SHALL report `dev` as its version.

#### Scenario: Local build
- **WHEN** the daemon is built and run locally without a version supplied
- **THEN** it reports version `dev`

### Requirement: Version Endpoint
The system SHALL expose `GET /api/version`, without requiring authentication, responding with HTTP 200 and a JSON body `{"version": "<version>"}` containing the running version.

#### Scenario: Requesting the version
- **WHEN** a client sends `GET /api/version` to a daemon running version `v0.9.2`
- **THEN** the daemon responds with HTTP 200 and the body `{"version":"v0.9.2"}`

### Requirement: Version Shown Next To The Logo
The web UI header SHALL show the running version as small, secondary text immediately after the logo, at all viewport sizes. Clicking the version SHALL NOT navigate anywhere.

#### Scenario: Version loaded
- **WHEN** the UI loads and `GET /api/version` returns `{"version":"v0.9.2"}`
- **THEN** the header shows `v0.9.2` right after the logo

#### Scenario: Clicking the version
- **WHEN** a user on a detail view clicks the version text
- **THEN** the application stays on the current view

### Requirement: Version Fails Silently
The web UI SHALL show no version text, and SHALL NOT show any error, while the version is loading or when its request fails. The rest of the UI SHALL keep working normally.

#### Scenario: Version still loading
- **WHEN** the `GET /api/version` request has not completed yet
- **THEN** no version text is shown in the header

#### Scenario: Version request fails
- **WHEN** `GET /api/version` responds with an error
- **THEN** no version text and no error message appear, and the rest of the UI renders normally
