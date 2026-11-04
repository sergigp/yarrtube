## ADDED Requirements

### Requirement: Summary Visibility
A stored summary SHALL be shown only while AI is on, summaries are on and the source holding that copy of the video has `summaries_enabled` `true`. Otherwise the system SHALL treat the video as having no summary wherever it is shown. A hidden summary SHALL stay stored and SHALL show again, without being regenerated, once all three conditions hold.

#### Scenario: AI turned off
- **WHEN** a video has a stored summary and AI is off
- **THEN** its summary is not shown and stays stored

#### Scenario: Summaries turned off
- **WHEN** a video has a stored summary, AI is on and summaries are off
- **THEN** its summary is not shown and stays stored

#### Scenario: Source opted out
- **WHEN** a video has a stored summary, AI and summaries are on, and its channel has `summaries_enabled` `false`
- **THEN** its summary is not shown and stays stored

#### Scenario: Source opted back in
- **WHEN** a channel with a hidden summary turns `summaries_enabled` back on
- **THEN** the summary is shown again and Ollama is not called

#### Scenario: Same video in two sources
- **WHEN** a YouTube video is tracked by a channel with `summaries_enabled` `true` and a playlist with it `false`, and both copies have a stored summary
- **THEN** the channel's copy shows its summary and the playlist's copy shows none
