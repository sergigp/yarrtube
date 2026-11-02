## ADDED Requirements

### Requirement: Quality Check Tasks Carry Video Context
The task listing SHALL resolve a quality check task's context the same way as a chapters fetch task's: the title of the video whose quality is being checked and the name of the playlist or channel it belongs to, when they can still be found.

#### Scenario: Task checks a specific video's quality
- **WHEN** a quality check task is pending for a video that still exists in a tracked playlist
- **THEN** the listed task has type `upgrade_quality` and includes that video's title and the playlist's name

#### Scenario: Quality check task whose video no longer exists
- **WHEN** a quality check task is pending for a video whose record was deleted
- **THEN** the task is still listed, without a video title
