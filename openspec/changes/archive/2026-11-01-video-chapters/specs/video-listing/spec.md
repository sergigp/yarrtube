# Spec Delta

## ADDED Requirements

### Requirement: Listed Videos Include Their Chapters
The system SHALL include each video's recorded chapters, in playback order, in the responses that list the videos of a playlist, of a channel and of a category: each chapter's start and end in whole seconds and its title. A video recorded as having no chapters, or whose chapters were never checked, SHALL be reported with an empty chapter list.

#### Scenario: Video with chapters is listed
- **WHEN** a client lists the videos of a tracked playlist, channel or category that has a video recorded with chapters "Game 1" (0–1037 s) and "Game 2" (1037–2201 s)
- **THEN** that video's entry includes exactly those two chapters, in that order, each with its start, end and title

#### Scenario: Video without chapters is listed
- **WHEN** a client lists the videos of a tracked playlist, channel or category that has a video recorded as having no chapters
- **THEN** that video's entry includes an empty chapter list

#### Scenario: Video with unchecked chapters is listed
- **WHEN** a client lists the videos of a tracked playlist, channel or category that has a video whose chapters were never checked
- **THEN** that video's entry includes an empty chapter list, and every other field is still returned
