## MODIFIED Requirements

### Requirement: Task That Changed Nothing Is Reported as Unchanged
The system SHALL let a task report that its run changed nothing, and SHALL then complete it like a done task, its end line reporting outcome `unchanged`. A `sync_video` task whose video needed nothing SHALL report this.

#### Scenario: Video sync with nothing to do
- **WHEN** a `sync_video` task runs for a video that has no step due, no broken file and no missing metadata (a legacy-layout video's never counts as missing)
- **THEN** the task is completed and removed, its end line reports outcome `unchanged`, and no `synced video` line is logged

#### Scenario: Video sync that does something
- **WHEN** a `sync_video` task schedules a due step, resets the video for redownload or repairs its metadata
- **THEN** one info line names the video and what the sync did, and the task's end line reports outcome `done`

