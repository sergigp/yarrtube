## MODIFIED Requirements

### Requirement: Settings Page
The application SHALL have a Settings page with the sections "General", "AI" and "Plex", in that order, at `/settings/general`, `/settings/ai` and `/settings/plex`. A section nav SHALL list them, marking the open one: a column beside the section on wide viewports, a row of tabs above it on mobile-width ones. Under each name the nav SHALL show the section's status: "Home and categories" for General; "Off", "On · Ollama" or "On · Anthropic" for AI, from the saved `ai_enabled` and `ai_provider`; and for Plex "Not set up" when Plex isn't configured, "Not checked yet" before its first sync, "Connected" when the latest sync reached it, or "Can't reach Plex" when it didn't. A status SHALL NOT show while its data is loading or failed to load. On mobile-width viewports the statuses SHALL NOT show. `/settings` SHALL open "General".

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

#### Scenario: Statuses
- **WHEN** AI is on with Anthropic and the latest Plex sync reached the server
- **THEN** the nav shows "On · Anthropic" under "AI" and "Connected" under "Plex"

#### Scenario: Plex not set up
- **WHEN** Plex isn't configured and AI is off
- **THEN** the nav shows "Off" under "AI" and "Not set up" under "Plex"

### Requirement: Preferences View
The Settings page's "General" section, at `/settings/general`, SHALL show its preferences as switches, each in a card with a heading. The "Categories" card SHALL hold a "Categories" switch. Under it SHALL be the description "Group channels and playlists by topic and browse by category." and an "Also used by" row of links: "AI categorization" to `/settings/ai`, "Plex category playlists" to `/settings/plex` and "Home: More about" to the "Home sections" card. The "Home sections" card SHALL hold "Continue watching", "More about…", "Quick watches" and "Latest videos", in that order. Toggling a switch SHALL save the preferences at once and update the whole application.

#### Scenario: Current values shown
- **WHEN** a user opens the General section and only `home_show_latest` is off
- **THEN** every switch is on except "Latest videos"

#### Scenario: Toggling saves
- **WHEN** a user turns off "Quick watches"
- **THEN** the preferences are saved with `home_show_quick_watches` off and the switch stays off after reloading the page

#### Scenario: Save fails
- **WHEN** a user toggles a switch and the save fails with "database is locked"
- **THEN** the view shows "Couldn't save: database is locked. Try again." and the switch returns to its saved value

#### Scenario: Following a cross-link
- **WHEN** a user activates "Plex category playlists"
- **THEN** the URL becomes `/settings/plex`

### Requirement: Preferences Help
Each preference switch SHALL show its description as text under its label, not in a tooltip, and SHALL have that description as its accessible description. The descriptions SHALL be:
- "Continue watching": "Videos you started in the last 7 days and haven't finished."
- "More about…": "One section for each of up to 3 categories with unwatched videos."
- "Quick watches": "Unwatched videos under 15 minutes."
- "Latest videos": "Your newest downloads that aren't in a section above."

The "Home sections" card SHALL say "Shown top to bottom in this order."

#### Scenario: Explaining a preference
- **WHEN** a user opens the General section
- **THEN** "Unwatched videos under 15 minutes." shows under "Quick watches" without any interaction

#### Scenario: Help on touch screens
- **WHEN** a user opens the General section
- **THEN** no "About <label>" control is shown

### Requirement: Preferences Save Confirmation
After a preference is saved, the General section SHALL show "Saved" next to its title, "General", for about 2 seconds. A failed save SHALL NOT show it. The section SHALL say "Changes save as you make them." under its title.

#### Scenario: Saved
- **WHEN** a user turns off "Quick watches" and the save succeeds
- **THEN** "Saved" shows next to "General" and is gone about 2 seconds later

#### Scenario: Not saved
- **WHEN** a user toggles a switch and the save fails
- **THEN** "Saved" does not show

### Requirement: Home Category Switch Needs Categories
While "Categories" is off, the "More about…" switch SHALL be disabled, keeping its saved value.

#### Scenario: Categories off
- **WHEN** a user turns off "Categories"
- **THEN** "More about…" is disabled and keeps its value
