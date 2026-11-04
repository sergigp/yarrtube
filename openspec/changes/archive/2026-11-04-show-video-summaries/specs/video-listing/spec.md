## ADDED Requirements

### Requirement: Listed Videos Include Their Summary
The system SHALL include each video's AI summary text in the responses that list the videos of a playlist, of a channel and of a category. It SHALL be reported as absent for a video with no stored summary, and for one whose summary is hidden (see `video-summaries`). In a category, whether it is hidden SHALL follow the source the video is returned from.

#### Scenario: Video with a summary is listed
- **WHEN** AI and summaries are on and a client lists the videos of a tracked playlist, channel or category whose source has summaries enabled and that has a video with a stored summary
- **THEN** that video's entry includes the summary's text

#### Scenario: Video without a summary is listed
- **WHEN** a client lists a video that has no stored summary, whether it is waiting for one or was given up on
- **THEN** that video's entry reports its summary as absent, and every other field is still returned

#### Scenario: Hidden summary is listed
- **WHEN** a client lists a video whose stored summary is hidden
- **THEN** that video's entry reports its summary as absent

#### Scenario: Category video returned from a channel that opted out
- **WHEN** a category lists a YouTube video tracked by a channel with `summaries_enabled` `false` and a playlist with it `true`
- **THEN** the video is returned from the channel and reports its summary as absent
