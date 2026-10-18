# Spec Delta

## MODIFIED Requirements

### Requirement: Task Lanes
The system SHALL execute tasks in lanes, each with its own maximum number of tasks running at the same time:
- a **download lane**, containing video downloads, whose concurrency is configurable,
- a **thumbnail lane**, containing thumbnail fetches, with a concurrency of 1,
- a **subtitle lane**, containing subtitle fetches, with a concurrency of 1,
- a **light lane**, containing every other task type except thumbnail fetches, subtitle fetches and the yt-dlp update (reconciles and file deletions), with a concurrency of 1.

A lane SHALL start a new eligible task whenever it has fewer tasks running than its concurrency. A long-running task in one lane SHALL NOT delay the start of eligible tasks in another lane.

#### Scenario: Downloads run in parallel up to the lane's concurrency
- **WHEN** the download lane's concurrency is 2 and three video downloads are eligible
- **THEN** two of them run at the same time, and the third starts as soon as one of the first two finishes

#### Scenario: Light tasks are not blocked by downloads
- **WHEN** the download lane is fully busy with long-running downloads and a reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any download to finish

#### Scenario: Light lane runs one task at a time
- **WHEN** two light-lane tasks are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

#### Scenario: Reconciles and deletes are not blocked by thumbnail fetches
- **WHEN** many thumbnail fetches are queued, one of them is running, and a reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any of the thumbnail fetches to finish

#### Scenario: Thumbnail lane runs one fetch at a time
- **WHEN** two thumbnail fetches for different videos are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes

#### Scenario: Subtitle backfill does not block other work
- **WHEN** many subtitle fetches are queued, one of them is running, and a video download, thumbnail fetch, reconcile or file-deletion task becomes eligible
- **THEN** that task starts without waiting for any of the subtitle fetches to finish

#### Scenario: Subtitle lane runs one fetch at a time
- **WHEN** two subtitle fetches for different videos are eligible at the same time
- **THEN** the system runs one, and starts the other only after the first finishes
