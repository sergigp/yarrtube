## ADDED Requirements

### Requirement: Parallel Workers
The suite SHALL run its tests on a configurable number of parallel workers, each driving its own daemon container with its own database, videos directory and Plex stub, so that no test sees another worker's library. The run script SHALL take the number of workers, defaulting to half the machine's CPU cores (at most 4) locally and to 2 in CI, and one worker SHALL behave like a sequential run.

#### Scenario: Several workers
- **WHEN** the suite runs with 3 workers
- **THEN** tests run three at a time, each against its own worker's daemon, and the run passes as it does with one worker

#### Scenario: Containers per run
- **WHEN** the suite runs with N workers
- **THEN** at most N daemon containers are alive at once, and each is removed when its worker ends

#### Scenario: Worker count chosen per environment
- **WHEN** a developer runs the suite without choosing a worker count, and CI runs it with its own
- **THEN** the developer's run uses half the CPU cores, at most 4, and CI uses the count it passed

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
