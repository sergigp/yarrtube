# Spec Delta

## MODIFIED Requirements

### Requirement: Source Categorization Setting
Every stored channel and playlist SHALL have a `categorization_enabled` boolean, `false` unless set. Sources stored before this setting existed SHALL have `false`. The setting SHALL decide whether the source's videos are evaluated by the AI (see `video-categorization`); it SHALL have no effect on syncing or downloading. Turning it off SHALL stop new evaluations and SHALL keep the categories the AI already gave its videos.

#### Scenario: Existing sources after upgrade
- **WHEN** the daemon starts on a database with channels and playlists stored before this setting existed
- **THEN** every one of them has `categorization_enabled` `false`

#### Scenario: Nothing is evaluated
- **WHEN** a channel with `categorization_enabled` `false` syncs a new video that gets a summary
- **THEN** the video is not evaluated and belongs only to the channel's categories

#### Scenario: Source opted in
- **WHEN** AI categorization is on, a channel has `categorization_enabled` `true`, and a new video of it gets a summary
- **THEN** the video is evaluated against every category with an AI prompt the channel isn't a member of, from its next sync after the summary (at the latest the channel's next reconcile pass)

#### Scenario: Turned off after evaluations
- **WHEN** a channel whose videos have AI categories has its categorization turned off
- **THEN** its videos keep those categories and its new videos are not evaluated
