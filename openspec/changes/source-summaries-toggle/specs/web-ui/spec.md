## ADDED Requirements

### Requirement: Summarize Videos Checkbox
The add and edit dialogs for channels and playlists SHALL show a "Summarize videos" checkbox described by the hint "Generate an AI summary of each new video.". In add dialogs it SHALL be always visible, below "Save to" and categories (not in "Advanced options"), unchecked by default, sent as `summaries_enabled`. Edit dialogs SHALL prefill it and send `summaries_enabled` only when changed.

#### Scenario: Visible without advanced options
- **WHEN** a user opens either add dialog without expanding "Advanced options"
- **THEN** "Summarize videos" is visible

#### Scenario: Adding a channel without touching it
- **WHEN** a user adds a channel or playlist without touching "Summarize videos"
- **THEN** it is created with `summaries_enabled` `false`

#### Scenario: Adding a playlist with summaries
- **WHEN** AI summaries are on and a user checks "Summarize videos" in the add playlist dialog and submits
- **THEN** the playlist is created with `summaries_enabled` `true`

#### Scenario: Edit dialog prefilled
- **WHEN** a user opens "Edit settings" for a channel whose summaries are enabled
- **THEN** "Summarize videos" is checked

#### Scenario: Enabling summaries from the edit dialog
- **WHEN** AI summaries are on and a user checks "Summarize videos" for a playlist and saves
- **THEN** the application sends `summaries_enabled` `true` alone, closes the dialog, and does not trigger a sync

### Requirement: Summarize Videos Needs AI Summaries
"Summarize videos" SHALL be enabled only while the preferences have both AI and summaries on. Otherwise, including while preferences load or fail to load, it SHALL be disabled unless the source already has summaries on, in which case it SHALL stay enabled so they can be turned off; its hint SHALL read "Turn on AI summaries in AI settings." with "AI settings" a link to `/ai` that closes the dialog.

#### Scenario: AI off
- **WHEN** AI is off and a user opens the add channel dialog
- **THEN** "Summarize videos" is unchecked and disabled, and the hint links to AI settings

#### Scenario: AI on but summaries off
- **WHEN** AI is on, summaries are off and a user opens "Edit settings" for a playlist
- **THEN** "Summarize videos" is disabled

#### Scenario: Turning summaries off while AI is off
- **WHEN** AI is off and a user opens "Edit settings" for a channel whose summaries are enabled, unchecks "Summarize videos" and saves
- **THEN** "Summarize videos" was shown checked and enabled, and only `summaries_enabled` `false` is sent

#### Scenario: Following the link
- **WHEN** AI is off and a user activates "AI settings" in the hint
- **THEN** the dialog closes and the AI page at `/ai` opens
