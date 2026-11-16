## MODIFIED Requirements

### Requirement: Categorize Videos Needs AI Categorization
"Categorize videos" SHALL be enabled only while the preferences have AI, summaries, categories and categorization all on. Otherwise, including while preferences load or fail to load, it SHALL be disabled unless the source already has categorization on, in which case it SHALL stay enabled so it can be turned off; its hint SHALL read "Turn on AI categorization in AI settings." with "AI settings" a link to `/settings/ai` that closes the dialog.

#### Scenario: AI off
- **WHEN** AI is off and a user opens the add channel dialog
- **THEN** "Categorize videos" is unchecked and disabled, and the hint links to AI settings

#### Scenario: Summaries off in the preferences
- **WHEN** AI, categories and categorization are on, summaries are off, and a user opens the add channel dialog
- **THEN** "Categorize videos" is unchecked and disabled, and the hint links to AI settings

#### Scenario: Turning categorization off while AI is off
- **WHEN** AI is off and a user opens "Edit settings" for a channel whose categorization is enabled, unchecks "Categorize videos" and saves
- **THEN** "Categorize videos" was shown checked and enabled, and only `categorization_enabled` `false` is sent

#### Scenario: Following the link
- **WHEN** AI is off and a user activates "AI settings" in the "Categorize videos" hint
- **THEN** the dialog closes and the AI section at `/settings/ai` opens

### Requirement: Re-run Or Clear From A Source's Menu
While categories are on, the actions menu of a channel or playlist SHALL offer "Re-run AI categorization" when AI categorization is available and the source has both summaries and categorization on, and "Clear AI categories" otherwise. Either SHALL ask for confirmation, naming the source, before deleting the source's AI verdicts.

#### Scenario: Source with categorization off
- **WHEN** the user opens the menu of "@GothamChess", whose categorization is off, and chooses "Clear AI categories", then confirms
- **THEN** the daemon is asked to delete the channel's AI verdicts

#### Scenario: Source with summaries off
- **WHEN** AI categorization is available and the user opens the menu of "@GothamChess", whose categorization is on and summaries off
- **THEN** the menu offers "Clear AI categories" and not "Re-run AI categorization"

#### Scenario: Cancelled
- **WHEN** the user chooses "Re-run AI categorization" and cancels the confirmation
- **THEN** nothing is sent to the daemon

## ADDED Requirements

### Requirement: Categorize Videos Needs Summarize Videos
In the add and the edit dialogs for channels and playlists, while AI categorization is available, "Categorize videos" SHALL be disabled while "Summarize videos" is unchecked in the dialog, keeping its checked state, with the hint "Needs Summarize videos.". The dialog SHALL still send `categorization_enabled` as checked: in add dialogs its value, in edit dialogs only when changed.

#### Scenario: Summarize unchecked in the add dialog
- **WHEN** AI summaries and categorization are on and a user opens the add channel dialog
- **THEN** "Categorize videos" is disabled with the hint "Needs Summarize videos." until the user checks "Summarize videos"

#### Scenario: Adding with both on
- **WHEN** AI summaries and categorization are on and a user checks "Summarize videos", then "Categorize videos", and submits the add playlist dialog
- **THEN** the playlist is created with `summaries_enabled` and `categorization_enabled` `true`

#### Scenario: Edit dialog of a source with summaries off
- **WHEN** AI summaries and categorization are on and a user opens "Edit settings" for a playlist with summaries and categorization off
- **THEN** "Categorize videos" is unchecked and disabled with the hint "Needs Summarize videos.", and becomes enabled once the user checks "Summarize videos"

#### Scenario: Unchecking summarize keeps categorize
- **WHEN** a user opens "Edit settings" for a channel with summaries and categorization on, unchecks "Summarize videos" and saves
- **THEN** "Categorize videos" stays checked and becomes disabled, and only `summaries_enabled` `false` is sent
