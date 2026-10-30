# Spec Delta

## ADDED Requirements

### Requirement: Home Sections Follow Preferences
The home endpoint SHALL return an empty list for each section turned off in the preferences: "Continue watching" when `home_show_continue_watching` is off, "Quick watches" when `home_show_quick_watches` is off, "Latest videos" when `home_show_latest` is off, and no category sections when `home_show_categories` or `categories_enabled` is off. A section turned off SHALL show no video, so videos it would have held can appear in the sections after it.

#### Scenario: Continue watching turned off
- **WHEN** `home_show_continue_watching` is off and an unwatched short video has been started recently
- **THEN** "Continue watching" is empty and the video appears under "Quick watches"

#### Scenario: Categories turned off
- **WHEN** `categories_enabled` is off and "Chess" has pending videos
- **THEN** the response holds no category sections, and Chess's videos can appear under "Quick watches" or "Latest videos"

#### Scenario: Home category sections turned off
- **WHEN** `categories_enabled` is on, `home_show_categories` is off and "Chess" has pending videos
- **THEN** the response holds no category sections

#### Scenario: Latest videos turned off
- **WHEN** `home_show_latest` is off
- **THEN** "Latest videos" is empty and the other sections are unchanged

#### Scenario: Everything on
- **WHEN** every preference is on
- **THEN** the home response is the same as without preferences
