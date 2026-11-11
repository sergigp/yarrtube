## MODIFIED Requirements

### Requirement: Summary Stored
A successful generation SHALL store the summary with the model that wrote it and when, set the summary state to Generated, and schedule a sync of the video to run now (see `video-sync`), so the steps that need a summary (categorization) don't wait for the next reconcile pass. A generated summary dropped because the video no longer wants one SHALL NOT schedule a sync. A video SHALL have at most one summary. A stored summary SHALL NOT be regenerated, including when the prompt or model changes or the video is redownloaded, and SHALL be kept when summaries are turned off.

#### Scenario: Summary generated
- **WHEN** Ollama answers with a summary
- **THEN** it is stored for the video with the model's name and the video's summary state is Generated

#### Scenario: Video synced once its summary is stored
- **WHEN** a video's summary is stored
- **THEN** a sync of that video is scheduled to run now

#### Scenario: Summary dropped
- **WHEN** summaries are turned off while a video's summary is being generated
- **THEN** nothing is stored and no sync of the video is scheduled

#### Scenario: Prompt changed later
- **WHEN** a video is Generated and the saved prompt changes
- **THEN** the video is not summarized again

#### Scenario: Video deleted during generation
- **WHEN** a video's record is deleted while its summary is being generated
- **THEN** nothing is stored
