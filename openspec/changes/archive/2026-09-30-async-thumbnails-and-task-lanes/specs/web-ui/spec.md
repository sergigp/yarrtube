## ADDED Requirements

### Requirement: Tasks View Describes Thumbnail Fetches
The tasks view SHALL describe a thumbnail fetch task in plain language,
naming the video whose thumbnail is being fetched and the playlist or
channel it belongs to, the same way it describes a video download task.

#### Scenario: Thumbnail fetch task with resolvable context
- **WHEN** the tasks view lists a thumbnail fetch task whose video title and playlist or channel name were resolved
- **THEN** the task is described as fetching the thumbnail of that video in that playlist or channel

#### Scenario: Thumbnail fetch task whose video can no longer be found
- **WHEN** the tasks view lists a thumbnail fetch task whose video title could not be resolved
- **THEN** the task is still described as a thumbnail fetch, using a generic placeholder in place of the missing names
