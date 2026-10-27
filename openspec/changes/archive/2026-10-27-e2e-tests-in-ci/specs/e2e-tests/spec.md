## MODIFIED Requirements

### Requirement: Continuous Integration
The system SHALL run the end-to-end suite automatically in CI on every pull request and on every push to `main`, together with the other test suites and without a YouTube API key; the upgrade check, which needs the real YouTube, is skipped there. A failing run SHALL keep its Playwright results and the container's log as artifacts of the run.

#### Scenario: Pull request opened
- **WHEN** a pull request is opened or updated
- **THEN** CI runs the end-to-end suite against an image built from the pull request and reports its result on the pull request

#### Scenario: Run fails
- **WHEN** an end-to-end test fails in CI
- **THEN** the run's Playwright traces and videos and the container's log are uploaded as artifacts
