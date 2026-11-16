## MODIFIED Requirements

### Requirement: When A Video Is Categorized
A video's sync SHALL schedule its categorization only when AI, summaries, categories and categorization are on in the preferences, the video's playlist or channel has both `summaries_enabled` and `categorization_enabled` `true`, the video's summary is Done, at least one category is pending for it and its categorization is Pending, Done, or Retrying and due (see `video-retries`). A categorization already pending or running for the video SHALL NOT be scheduled again. Otherwise the sync SHALL schedule nothing for categorization. Storing a video's summary SHALL sync the video right away (see "Summary Stored" in `video-summaries`), so a newly summarized video is categorized without waiting for its source's next reconcile pass. A categorization that runs while these no longer hold SHALL be skipped without asking the model.

#### Scenario: Video with a summary and pending categories
- **WHEN** a video of a channel with summaries and categorization enabled has a Done summary and "Sicilian" is pending for it, and its sync runs
- **THEN** a categorization of that video is scheduled

#### Scenario: Video without a summary
- **WHEN** a video of a channel with categorization enabled has a Pending summary and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Nothing pending
- **WHEN** a video has a verdict for every category with an AI prompt and its sync runs
- **THEN** no categorization is scheduled and the model is not asked anything

#### Scenario: Source with categorization off
- **WHEN** a video of a playlist with `categorization_enabled` `false` has a Done summary and pending categories, and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Source with summaries off
- **WHEN** a video of a playlist with `categorization_enabled` `true` and `summaries_enabled` `false` has a Done summary and pending categories, and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Summaries off in the preferences
- **WHEN** AI, categories and categorization are on but summaries are off, and a video of a channel with summaries and categorization enabled has a Done summary and pending categories, and its sync runs
- **THEN** no categorization is scheduled

#### Scenario: Summaries turned off before a queued categorization runs
- **WHEN** a video's categorization is queued and summaries are turned off in the preferences before it runs
- **THEN** the run is skipped and the model is not asked

#### Scenario: Summary generated after the video's sync
- **WHEN** a new video's summary is stored after its sync ran, and "Sicilian" is pending for it
- **THEN** the video is synced again right away and that sync schedules its categorization

#### Scenario: Categorization failed recently
- **WHEN** a video with pending categories whose categorization is Retrying, having failed once 3 minutes ago, is synced with no categorization pending
- **THEN** no categorization is scheduled
