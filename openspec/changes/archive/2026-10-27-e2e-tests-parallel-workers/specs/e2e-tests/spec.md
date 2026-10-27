## ADDED Requirements

### Requirement: Parallel Workers
The suite SHALL run its tests on a configurable number of parallel workers, each driving its own daemon container with its own database, videos directory and Plex stub, so that no test sees another worker's library. The run script SHALL take the number of workers, defaulting to 3 locally and in CI alike, and one worker SHALL behave like a sequential run.

#### Scenario: Several workers
- **WHEN** the suite runs with 3 workers
- **THEN** tests run three at a time, each against its own worker's daemon, and the run passes as it does with one worker

#### Scenario: Containers per run
- **WHEN** the suite runs with N workers
- **THEN** at most N daemon containers are alive at once, and each is removed when its worker ends

#### Scenario: Default worker count
- **WHEN** the suite runs without a worker count, locally or in CI
- **THEN** it runs on 3 workers

#### Scenario: Worker count chosen by the developer
- **WHEN** a developer runs the suite with `--workers 1`
- **THEN** it runs one test at a time

### Requirement: Run Summary
The test script SHALL end every run, passing or failing, with a summary listing each phase it ran (the web UI checks, Rust formatting, clippy and tests, the contract tests, the end-to-end image build, upgrade check and suite) with its duration and, for phases that run tests, how many passed (and were ignored, flaky, skipped or failed), each suite it skipped, and the total duration.

#### Scenario: A passing run
- **WHEN** the script finishes with every selected suite passing
- **THEN** its last lines list each phase's time and test counts, the skipped suites and the total time

#### Scenario: A failing run
- **WHEN** a phase fails
- **THEN** the summary still prints, with the phases that ran and the failed one marked as failed

## MODIFIED Requirements

### Requirement: Isolated Runs
The suite's run script SHALL give each run its own container names, image tag and ports, and each of its workers its own daemon container on ports of its own, so that runs started from different worktrees at the same time neither stop each other's containers nor collide on ports. It SHALL keep each worker's container log with that worktree's test results, and SHALL fail the run when any daemon logged a panic.

#### Scenario: Two worktrees at once
- **WHEN** the suite is started from two worktrees at the same time
- **THEN** each run starts and removes only its own containers, its services listen on ports of their own, and both runs complete

#### Scenario: Ports chosen by the developer
- **WHEN** a developer sets the suite's port variables
- **THEN** the shared fake YouTube and the upgrade check's container use those ports instead of free ones, while each worker's daemon still takes free ports

#### Scenario: A daemon panics
- **WHEN** a worker's daemon logs a panic during the run
- **THEN** the run fails, and that worker's container log is in the test results
