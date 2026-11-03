# Spec Delta

## MODIFIED Requirements

### Requirement: Header Settings Menu
The header SHALL offer, at its trailing edge, a settings control shown as a gear icon with the accessible name "Settings". Activating it SHALL open a menu with, in order, "Tasks", which navigates to the tasks view, "Preferences", which navigates to the preferences view, "AI", which navigates to the AI page, and "Report a bug", which opens `https://github.com/sergigp/yarrtube/issues/new` in a new browser tab. The header SHALL NOT show any other control for adding channels or playlists or for reaching the tasks view.

#### Scenario: Reaching the tasks view
- **WHEN** a user activates the header's settings control and chooses "Tasks"
- **THEN** the application navigates to the tasks view

#### Scenario: Reaching the preferences view
- **WHEN** a user activates the header's settings control and chooses "Preferences"
- **THEN** the application navigates to the preferences view

#### Scenario: Reaching the AI page
- **WHEN** a user activates the header's settings control and chooses "AI"
- **THEN** the application navigates to the AI page

#### Scenario: Reporting a bug
- **WHEN** a user activates the header's settings control and chooses "Report a bug"
- **THEN** the GitHub new issue page for yarrtube opens in a new tab and the application stays where it was

#### Scenario: Header controls
- **WHEN** a user views any page
- **THEN** the header shows the settings control and no "Add" or "Tasks" button
