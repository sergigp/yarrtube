# Spec Delta

## ADDED Requirements

### Requirement: Tasks View Describes Chapter Fetches
The tasks view SHALL describe a chapters fetch task in plain language, naming the video whose chapters are being fetched and the playlist or channel it belongs to, and SHALL list it with the download tasks, the same way it describes and lists a subtitle fetch task.

#### Scenario: Chapters fetch task with resolvable context
- **WHEN** the tasks view lists a chapters fetch task whose video title and playlist or channel name were resolved
- **THEN** the task is described as fetching the chapters of that video in that playlist or channel, under the downloads tab

#### Scenario: Chapters fetch task whose video can no longer be found
- **WHEN** the tasks view lists a chapters fetch task whose video title could not be resolved
- **THEN** the task is still described as a chapters fetch, using a generic placeholder in place of the missing names
