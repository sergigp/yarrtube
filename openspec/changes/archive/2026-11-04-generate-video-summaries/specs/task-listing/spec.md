# Spec Delta

## ADDED Requirements

### Requirement: Summary Tasks Carry Video Context
The task listing SHALL resolve a video summary task's context the same way as a chapters fetch task's: the title of the video being summarized and the name of the playlist or channel it belongs to, when they can still be found.

#### Scenario: Task summarizes a specific video
- **WHEN** a summary task is pending for a video that still exists in a tracked channel
- **THEN** the listed task has type `summarize_video` and includes that video's title and the channel's name

#### Scenario: Summary task whose video no longer exists
- **WHEN** a summary task is pending for a video whose record was deleted
- **THEN** the task is still listed, without a video title
