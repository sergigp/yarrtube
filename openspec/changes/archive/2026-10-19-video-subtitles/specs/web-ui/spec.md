# Spec Delta

## ADDED Requirements

### Requirement: Tasks View Describes Subtitle Fetches
The tasks view SHALL describe a subtitle fetch task in plain language, naming the video whose subtitles are being fetched and the playlist or channel it belongs to, and SHALL list it with the download tasks, the same way it describes and lists a thumbnail fetch task.

#### Scenario: Subtitle fetch task with resolvable context
- **WHEN** the tasks view lists a subtitle fetch task whose video title and playlist or channel name were resolved
- **THEN** the task is described as fetching the subtitles of that video in that playlist or channel, under the downloads tab

#### Scenario: Subtitle fetch task whose video can no longer be found
- **WHEN** the tasks view lists a subtitle fetch task whose video title could not be resolved
- **THEN** the task is still described as a subtitle fetch, using a generic placeholder in place of the missing names
