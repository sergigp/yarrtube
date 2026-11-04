## ADDED Requirements

### Requirement: Video Detail AI Summary
The video detail pane SHALL show the video's summary, when the listing reports one, in full between the meta line and the storage location, under the label "AI summary", at every viewport size. Line breaks in the summary SHALL be kept. When the listing reports none, the pane SHALL show neither the text nor the label.

#### Scenario: Video with a summary
- **WHEN** a user selects a video in a playlist, channel or category view whose listing entry has a summary
- **THEN** the detail pane shows "AI summary" and the whole summary below the meta line, with no control to expand it

#### Scenario: Video without a summary
- **WHEN** a user selects a video whose listing entry has no summary
- **THEN** the detail pane shows no "AI summary" label and no placeholder

### Requirement: Saved Preferences Refresh Video Lists
Saving preferences SHALL refetch the playlist, channel and category video lists, including those not on screen, so returning to one doesn't first show a summary that was turned off.

#### Scenario: Turning AI off and going back
- **WHEN** a user viewing a video with a summary goes to AI settings, turns AI off and returns to the video
- **THEN** the detail pane shows no summary once the list has refetched

## MODIFIED Requirements

### Requirement: Summarize Videos Checkbox
The add and edit dialogs for channels and playlists SHALL show a "Summarize videos" checkbox described by the hint "Generate and show an AI summary of each video.". In add dialogs it SHALL be always visible, below "Save to" and categories (not in "Advanced options"), unchecked by default, sent as `summaries_enabled`. Edit dialogs SHALL prefill it and send `summaries_enabled` only when changed.

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
