## ADDED Requirements

### Requirement: Tasks View Describes Quality Checks
The tasks view SHALL describe a quality check task in plain language, naming the video whose quality is being checked and the playlist or channel it belongs to, and SHALL list it with the download tasks, the same way it describes and lists a chapters fetch task.

#### Scenario: Quality check task with resolvable context
- **WHEN** the tasks view lists a quality check task whose video title and playlist or channel name were resolved
- **THEN** the task is described as checking the quality of that video in that playlist or channel, under the downloads tab

#### Scenario: Quality check task whose video can no longer be found
- **WHEN** the tasks view lists a quality check task whose video title could not be resolved
- **THEN** the task is still described as a quality check, using a generic placeholder in place of the missing names
