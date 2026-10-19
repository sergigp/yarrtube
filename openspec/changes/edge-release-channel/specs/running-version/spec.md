## MODIFIED Requirements

### Requirement: Edge Builds Carry Their Commit
A container image built for a merge to `main`, or by a manual release run, SHALL report `edge-<short commit sha>` as its version and SHALL report channel `edge`.

#### Scenario: Merge to main
- **WHEN** commit `abc1234` is merged to `main` and its edge image is built
- **THEN** a daemon running that image reports version `edge-abc1234` and channel `edge`

#### Scenario: Manual edge build
- **WHEN** the release workflow is run manually on commit `ccc3333`
- **THEN** a daemon running that image reports version `edge-ccc3333` and channel `edge`

### Requirement: Unversioned Builds Report Dev
A binary built without a release version (e.g. a local build) SHALL report `dev` as its version, channel `dev`, and no commit.

#### Scenario: Local build
- **WHEN** the daemon is built and run locally without a version supplied
- **THEN** it reports version `dev`, channel `dev`, and no commit

### Requirement: Version Endpoint
The system SHALL expose `GET /api/version` without requiring authentication. It SHALL respond with HTTP 200 and a JSON body `{"version": "<version>", "channel": "release" | "edge" | "dev", "commit": {"sha": "<full sha>", "subject": "<subject line>", "date": "<RFC 3339 commit date>"} | null}`.

#### Scenario: Requesting the version
- **WHEN** a client sends `GET /api/version` to a daemon built from commit `abc1234…` with subject `Fix sync` committed at `2026-10-19T10:00:00Z`
- **THEN** the daemon responds with HTTP 200 and the body `{"version":"edge-abc1234","channel":"edge","commit":{"sha":"abc1234…","subject":"Fix sync","date":"2026-10-19T10:00:00Z"}}`

#### Scenario: Requesting the version of a local build
- **WHEN** a client sends `GET /api/version` to a locally built daemon
- **THEN** the daemon responds with HTTP 200 and the body `{"version":"dev","channel":"dev","commit":null}`

### Requirement: Version Shown Next To The Logo
The web UI header SHALL show the running build immediately after the logo, at all viewport sizes. For `release` and `dev` builds it SHALL show the version as small, secondary text. For `edge` builds it SHALL show an `edge` badge instead of the version text. Clicking the version text or the badge SHALL NOT navigate anywhere.

#### Scenario: Version loaded
- **WHEN** the UI loads and `GET /api/version` returns version `v0.9.5` with channel `release`
- **THEN** the header shows `v0.9.5` right after the logo

#### Scenario: Edge build loaded
- **WHEN** the UI loads and `GET /api/version` returns channel `edge`
- **THEN** the header shows an `edge` badge right after the logo and no version text

#### Scenario: Clicking the version
- **WHEN** a user on a detail view clicks the version text or the edge badge
- **THEN** the application stays on the current view

## ADDED Requirements

### Requirement: Release Builds Report Their Tag
A container image built for a release SHALL report the release tag it was built from (e.g. `v0.9.5`) as its version, independently of the version declared in the package manifest, and SHALL report channel `release`.

#### Scenario: Image built for a pushed release tag
- **WHEN** a `v1.0.0` tag is pushed and its release image is built
- **THEN** a daemon running that image reports version `v1.0.0` and channel `release`


### Requirement: Edge Badge Shows Its Commit
Activating the `edge` badge SHALL work by mouse click, touch or keyboard. It SHALL open a popover showing:
- the short (7-character) commit sha, as a link that opens `https://github.com/sergigp/yarrtube/commit/<full sha>` in a new tab
- the commit subject
- the commit date

The popover SHALL close on outside click or Escape.

#### Scenario: Opening the popover
- **WHEN** a user clicks the `edge` badge of a build from commit `abc1234…` with subject `Fix sync`
- **THEN** a popover shows the link `abc1234` pointing to that commit on GitHub, the text `Fix sync`, and the commit date

#### Scenario: Closing the popover
- **WHEN** the popover is open and the user presses Escape
- **THEN** the popover closes

## REMOVED Requirements

### Requirement: Release Builds Carry Their Release Tag
**Reason**: Releases are no longer tagged automatically on every merge, so its "automatically tagged release" scenario no longer holds.
**Migration**: Replaced by "Release Builds Report Their Tag".
