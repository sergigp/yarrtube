# task-listing Specification

## Purpose

Provides an HTTP endpoint to list tasks that have not yet completed, so pending and in-progress work (playlist reconciliation, video downloads, file cleanup) can be watched from a browser.

## Requirements

### Requirement: List Non-Completed Tasks
The system SHALL provide an HTTP endpoint that returns every scheduled task whose status is `pending` or `running`, regardless of when it is next due to run, including each task's type, status, retry count, and scheduled run time.

#### Scenario: Non-completed tasks exist
- **WHEN** a client requests the list of non-completed tasks and one or more tasks are `pending` or `running`
- **THEN** the daemon responds with HTTP status 200 and a list containing each of those tasks

#### Scenario: No non-completed tasks exist
- **WHEN** a client requests the list of non-completed tasks and none are currently `pending` or `running`
- **THEN** the daemon responds with HTTP status 200 and an empty list

#### Scenario: A pending task is not yet due
- **WHEN** a client requests the list of non-completed tasks and a `pending` task's scheduled run time is in the future
- **THEN** that task is still included in the response
