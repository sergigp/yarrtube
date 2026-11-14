# Spec Delta

## ADDED Requirements

### Requirement: Settings Page
The application SHALL have a Settings page with the sections "General", "AI" and "Plex", in that order, at `/settings/general`, `/settings/ai` and `/settings/plex`. A section nav SHALL list them, marking the open one: a column beside the section on wide viewports, a row of tabs above it on mobile-width ones. `/settings` SHALL open "General".

#### Scenario: Switching sections
- **WHEN** a user on `/settings/general` activates "Plex" in the section nav
- **THEN** the URL becomes `/settings/plex`, the Plex section shows and "Plex" is marked as open

#### Scenario: Bare settings route
- **WHEN** a user opens `/settings`
- **THEN** the URL becomes `/settings/general` and the General section shows

#### Scenario: Back button
- **WHEN** a user goes from `/settings/general` to `/settings/ai` and presses back
- **THEN** the General section shows again

#### Scenario: Mobile width
- **WHEN** a user opens `/settings/ai` on a mobile-width viewport
- **THEN** the sections show as tabs above the AI section, with no horizontal page scroll

### Requirement: Old Settings Routes Redirect
`/preferences` SHALL redirect to `/settings/general` and `/ai` to `/settings/ai`, replacing the history entry.

#### Scenario: Old bookmark
- **WHEN** a user opens `/ai`
- **THEN** the URL becomes `/settings/ai` and the AI section shows

## MODIFIED Requirements

### Requirement: Header Settings Menu
The header SHALL offer, at its trailing edge, a settings control shown as a gear icon with the accessible name "Settings". Activating it SHALL open a menu with, in order, "Tasks", which navigates to the tasks view, "Settings", which navigates to the Settings page at `/settings/general`, and "Report a bug", which opens `https://github.com/sergigp/yarrtube/issues/new` in a new browser tab. The header SHALL NOT show any other control for adding channels or playlists or for reaching the tasks view.

#### Scenario: Reaching the tasks view
- **WHEN** a user activates the header's settings control and chooses "Tasks"
- **THEN** the application navigates to the tasks view

#### Scenario: Reaching the preferences view
- **WHEN** a user activates the header's settings control and chooses "Settings"
- **THEN** the application navigates to `/settings/general`, the Settings page's General section

#### Scenario: Reaching the AI page
- **WHEN** a user chooses "Settings" in the header's settings menu and then "AI" in the section nav
- **THEN** the application shows the AI section at `/settings/ai`

#### Scenario: No separate preferences or AI entries
- **WHEN** a user opens the header's settings menu
- **THEN** it lists "Tasks", "Settings" and "Report a bug" only

#### Scenario: Reporting a bug
- **WHEN** a user activates the header's settings control and chooses "Report a bug"
- **THEN** the GitHub new issue page for yarrtube opens in a new tab and the application stays where it was

#### Scenario: Header controls
- **WHEN** a user views any page
- **THEN** the header shows the settings control and no "Add" or "Tasks" button

### Requirement: Preferences View
The Settings page's "General" section, at `/settings/general`, SHALL show a labelled switch per preference: "Enable categories" in a "Categories" group, and "Show continue watching", "Show categories", "Show quick watches" and "Show latest videos" in a "Home" group. Toggling a switch SHALL save the preferences at once and update the whole application.

#### Scenario: Current values shown
- **WHEN** a user opens the General section and only `home_show_latest` is off
- **THEN** every switch is on except "Show latest videos"

#### Scenario: Toggling saves
- **WHEN** a user turns off "Show quick watches"
- **THEN** the preferences are saved with `home_show_quick_watches` off and the switch stays off after reloading the page

#### Scenario: Save fails
- **WHEN** a user toggles a switch and the save fails
- **THEN** the view shows the error and the switch returns to its saved value

### Requirement: Preferences Save Confirmation
After a preference is saved, the General section SHALL show "Saved" next to its title, "General", for about 2 seconds. A failed save SHALL NOT show it.

#### Scenario: Saved
- **WHEN** a user turns off "Show quick watches" and the save succeeds
- **THEN** "Saved" shows next to "General" and is gone about 2 seconds later

#### Scenario: Not saved
- **WHEN** a user toggles a switch and the save fails
- **THEN** "Saved" does not show

### Requirement: Summarize Videos Needs AI Summaries
"Summarize videos" SHALL be enabled only while the preferences have both AI and summaries on. Otherwise, including while preferences load or fail to load, it SHALL be disabled unless the source already has summaries on, in which case it SHALL stay enabled so they can be turned off; its hint SHALL read "Turn on AI summaries in AI settings." with "AI settings" a link to `/settings/ai` that closes the dialog.

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
- **THEN** the dialog closes and the AI section at `/settings/ai` opens

### Requirement: Categorize Videos Needs AI Categorization
"Categorize videos" SHALL be enabled only while the preferences have AI, categories and categorization all on. Otherwise, including while preferences load or fail to load, it SHALL be disabled unless the source already has categorization on, in which case it SHALL stay enabled so it can be turned off; its hint SHALL read "Turn on AI categorization in AI settings." with "AI settings" a link to `/settings/ai` that closes the dialog.

#### Scenario: AI off
- **WHEN** AI is off and a user opens the add channel dialog
- **THEN** "Categorize videos" is unchecked and disabled, and the hint links to AI settings

#### Scenario: Turning categorization off while AI is off
- **WHEN** AI is off and a user opens "Edit settings" for a channel whose categorization is enabled, unchecks "Categorize videos" and saves
- **THEN** "Categorize videos" was shown checked and enabled, and only `categorization_enabled` `false` is sent

#### Scenario: Following the link
- **WHEN** AI is off and a user activates "AI settings" in the "Categorize videos" hint
- **THEN** the dialog closes and the AI section at `/settings/ai` opens
