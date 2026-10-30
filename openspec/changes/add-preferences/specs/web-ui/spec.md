# Spec Delta

## MODIFIED Requirements

### Requirement: Header Settings Menu
The header SHALL offer, at its trailing edge, a settings control shown as a gear icon with the accessible name "Settings". Activating it SHALL open a menu with, in order, "Tasks", which navigates to the tasks view, "Preferences", which navigates to the preferences view, and "Report a bug", which opens `https://github.com/sergigp/yarrtube/issues/new` in a new browser tab. The header SHALL NOT show any other control for adding channels or playlists or for reaching the tasks view.

#### Scenario: Reaching the tasks view
- **WHEN** a user activates the header's settings control and chooses "Tasks"
- **THEN** the application navigates to the tasks view

#### Scenario: Reaching the preferences view
- **WHEN** a user activates the header's settings control and chooses "Preferences"
- **THEN** the application navigates to the preferences view

#### Scenario: Reporting a bug
- **WHEN** a user activates the header's settings control and chooses "Report a bug"
- **THEN** the GitHub new issue page for yarrtube opens in a new tab and the application stays where it was

#### Scenario: Header controls
- **WHEN** a user views any page
- **THEN** the header shows the settings control and no "Add" or "Tasks" button

## ADDED Requirements

### Requirement: Preferences View
The application SHALL have a "Preferences" view at `/preferences` showing a labelled switch per preference: "Enable categories" in a "Categories" group, and "Show continue watching", "Show categories", "Show quick watches" and "Show latest videos" in a "Home" group. Toggling a switch SHALL save the preferences at once and update the whole application.

#### Scenario: Current values shown
- **WHEN** a user opens the preferences view and only `home_show_latest` is off
- **THEN** every switch is on except "Show latest videos"

#### Scenario: Toggling saves
- **WHEN** a user turns off "Show quick watches"
- **THEN** the preferences are saved with `home_show_quick_watches` off and the switch stays off after reloading the page

#### Scenario: Save fails
- **WHEN** a user toggles a switch and the save fails
- **THEN** the view shows the error and the switch returns to its saved value

### Requirement: Home Category Switch Needs Categories
While "Enable categories" is off, the "Show categories" switch SHALL be disabled, keeping its saved value.

#### Scenario: Categories off
- **WHEN** a user turns off "Enable categories"
- **THEN** "Show categories" is disabled and keeps its value

### Requirement: No Category UI While Categories Are Off
While `categories_enabled` is off, or the preferences are still loading, the application SHALL NOT show the sidebar Categories section, the Categories field in the add and edit channel and playlist dialogs, home category sections or home card tints, and the sidebar search field SHALL be labelled for channels and playlists only. When the preferences cannot be loaded, the application SHALL behave as with the defaults.

#### Scenario: Sidebar without categories
- **WHEN** categories are off and "Chess" exists
- **THEN** the sidebar shows no Categories section and no "Add category" control

#### Scenario: Dialogs without categories
- **WHEN** categories are off and a user opens the add channel dialog
- **THEN** the dialog shows no Categories field, and saving it changes no category

#### Scenario: Preferences fail to load
- **WHEN** the preferences request fails
- **THEN** the sidebar shows the Categories section as with the defaults

### Requirement: Category View While Categories Are Off
While `categories_enabled` is off, a category view address SHALL say "Categories are turned off." with a link to the preferences view instead of the category.

#### Scenario: Category address while off
- **WHEN** categories are off and a user opens the "Chess" view's address
- **THEN** the view says "Categories are turned off." and links to the preferences view

### Requirement: Home Without Latest Videos
While `home_show_latest` is off, the home view SHALL NOT show "Latest videos". When no shown section has videos, the home view SHALL say "Nothing to show here." with a link to the preferences view.

#### Scenario: Latest off with other sections
- **WHEN** `home_show_latest` is off and "Continue watching" has videos
- **THEN** the home view shows "Continue watching" and no "Latest videos" heading

#### Scenario: Nothing to show
- **WHEN** `home_show_latest` is off and every other section is empty or off
- **THEN** the home view says "Nothing to show here." and links to the preferences view
