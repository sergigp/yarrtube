## MODIFIED Requirements

### Requirement: Source Categorization Setting
Every stored channel and playlist SHALL have a `categorization_enabled` boolean, `false` unless set. Sources stored before this setting existed SHALL have `false`. The setting SHALL decide whether the source's videos are evaluated by the AI (see `video-categorization`); it SHALL have no effect on syncing or downloading. It SHALL take effect only while the source's `summaries_enabled` is `true` too; while that is `false` the stored value SHALL be kept unchanged but no video of the source is evaluated. Turning it off SHALL stop new evaluations and SHALL keep the categories the AI already gave its videos.

#### Scenario: Existing sources after upgrade
- **WHEN** the daemon starts on a database with channels and playlists stored before this setting existed
- **THEN** every one of them has `categorization_enabled` `false`

#### Scenario: Nothing is evaluated
- **WHEN** a channel with `categorization_enabled` `false` syncs a new video that gets a summary
- **THEN** the video is not evaluated and belongs only to the channel's categories

#### Scenario: Source opted in
- **WHEN** AI summaries and AI categorization are on, a channel has `summaries_enabled` and `categorization_enabled` `true`, and a new video of it gets a summary
- **THEN** the video is evaluated against every category with an AI prompt the channel isn't a member of, from its next sync after the summary (at the latest the channel's next reconcile pass)

#### Scenario: Turned off after evaluations
- **WHEN** a channel whose videos have AI categories has its categorization turned off
- **THEN** its videos keep those categories and its new videos are not evaluated

#### Scenario: Source summaries off
- **WHEN** a channel has `categorization_enabled` `true` and `summaries_enabled` `false`, and one of its videos holds a Done summary from before and pending categories
- **THEN** the video is not evaluated, and the channel is still returned with `categorization_enabled` `true`

#### Scenario: Source summaries back on
- **WHEN** that channel's `summaries_enabled` is set back to `true`
- **THEN** its summarized videos with pending categories are evaluated again, without `categorization_enabled` being sent

### Requirement: Categorization Needs AI Categorization On
A create or update request for a channel or playlist that supplies `categorization_enabled` `true` SHALL be rejected with a conflict response, changing nothing, unless the preferences have AI, summaries, categories and categorization all on. The error SHALL read `Turn on AI categorization before enabling it for a channel` (or `for a playlist`). On creation the check SHALL run before YouTube is looked up. `categorization_enabled` `false` SHALL always be accepted. The source's own `summaries_enabled` SHALL NOT be checked.

#### Scenario: Creating with categorization while AI is off
- **WHEN** AI is off and a channel creation request supplies `categorization_enabled` `true`
- **THEN** the system stores nothing, does not look the channel up on YouTube, and returns a conflict with `Turn on AI categorization before enabling it for a channel`

#### Scenario: Enabling while categorization is off
- **WHEN** AI is on, categorization is off and a playlist update supplies `categorization_enabled` `true` and a new reconcile interval
- **THEN** the system changes nothing and returns a conflict with `Turn on AI categorization before enabling it for a playlist`

#### Scenario: Enabling while categories are off
- **WHEN** AI and categorization are on, categories are off and a channel update supplies `categorization_enabled` `true`
- **THEN** the system changes nothing and returns a conflict with `Turn on AI categorization before enabling it for a channel`

#### Scenario: Enabling while summaries are off
- **WHEN** AI, categories and categorization are on, summaries are off and a channel update supplies `categorization_enabled` `true`
- **THEN** the system changes nothing and returns a conflict with `Turn on AI categorization before enabling it for a channel`

#### Scenario: Enabling on a source with its summaries off
- **WHEN** AI, summaries, categories and categorization are on and a playlist with `summaries_enabled` `false` is updated with `categorization_enabled` `true`
- **THEN** the system stores the playlist with `categorization_enabled` `true` and returns it

#### Scenario: Disabling while AI is off
- **WHEN** AI is off and an update supplies `categorization_enabled` `false` for a channel whose categorization is on
- **THEN** the system stores the channel with `categorization_enabled` `false` and returns it

### Requirement: Categorization Setting On Creation
Creating a channel or a playlist SHALL require a `categorization_enabled` boolean. A missing or non-boolean value SHALL be rejected with a client error and nothing stored. Creating a source that already exists SHALL ignore the request's value and return the stored record.

#### Scenario: Created with categorization off
- **WHEN** a valid channel or playlist creation request supplies `categorization_enabled` `false`
- **THEN** the system stores the source with `categorization_enabled` `false` and returns it with `categorization_enabled` `false`

#### Scenario: Created without the setting
- **WHEN** a channel or playlist creation request omits `categorization_enabled`
- **THEN** the system stores nothing and rejects the request with a client error `Categorization setting must be true or false (missing)`

#### Scenario: Created with categorization on
- **WHEN** AI, summaries, categories and categorization are on and a valid channel or playlist creation request supplies `categorization_enabled` `true`
- **THEN** the system stores the source with `categorization_enabled` `true` and returns it with `categorization_enabled` `true`

#### Scenario: Duplicate source with a different setting
- **WHEN** AI, summaries, categories and categorization are on and a creation request names a channel or playlist already stored with `categorization_enabled` `false`, supplying `true`
- **THEN** the system makes no change and returns the existing record with `categorization_enabled` `false`

