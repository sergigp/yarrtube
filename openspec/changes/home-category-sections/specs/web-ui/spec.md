# Spec Delta

## MODIFIED Requirements

### Requirement: Home Page Layout
The home view SHALL display its sections in this order: "Continue watching", up to 3 category sections, "Quick watches" and "Latest videos", loaded together in one request. A video SHALL appear in at most one section. "Latest videos" SHALL display up to 18 recently synced videos. "Continue watching" SHALL display up to 6 videos the user has started and not finished recently. "Quick watches" SHALL display up to 6 short unwatched videos. Each category section SHALL display up to 6 videos, as described in "Home Category Sections". "Continue watching", the category sections and "Quick watches" SHALL be hidden when they have no videos, including while their videos are loading or when they cannot be loaded. Each section SHALL display its videos as a grid of cards, each showing the video's thumbnail with its title below it. For videos from a tracked channel, the card SHALL also show the channel's avatar and name.

#### Scenario: Home view with recent videos
- **WHEN** one or more videos have been synced
- **THEN** the home view renders them under "Latest videos" as a grid of thumbnail-and-title cards

#### Scenario: Channel video card
- **WHEN** a video in any home section comes from a tracked channel
- **THEN** its card shows the channel's avatar and name below the title

#### Scenario: Playlist video card
- **WHEN** a video in any home section comes from a tracked playlist
- **THEN** its card shows no channel avatar or name

#### Scenario: Continue watching shown first
- **WHEN** at least one video has been started and not finished recently
- **THEN** the home view shows a "Continue watching" section above the other sections, listing that video

#### Scenario: Continue watching card shows progress
- **WHEN** a video is shown under "Continue watching"
- **THEN** its thumbnail shows a progress bar proportional to its saved playback position over its duration

#### Scenario: Quick watches shown above latest videos
- **WHEN** at least one short unwatched video has been downloaded and is not shown in a category section
- **THEN** the home view shows a "Quick watches" section below "Continue watching" and the category sections, and above "Latest videos", listing that video

#### Scenario: Sections show a bounded number of videos
- **WHEN** more videos qualify for a section than it shows
- **THEN** "Continue watching", each category section and "Quick watches" each show at most 6 videos, and "Latest videos" shows at most 18

#### Scenario: A video appears in one section only
- **WHEN** a video qualifies for more than one home section
- **THEN** it is shown only in the first of them, in section order

#### Scenario: Empty sections are hidden
- **WHEN** no video qualifies for "Continue watching" or "Quick watches"
- **THEN** that section, including its heading, is not shown

#### Scenario: Optional sections that fail to load are hidden
- **WHEN** the videos for "Continue watching", the category sections or "Quick watches" are still loading or cannot be loaded
- **THEN** that section, including its heading, is not shown, and "Latest videos" still renders (showing the load error when there is one)

## ADDED Requirements

### Requirement: Home Category Sections
Each category section on the home view SHALL be headed "More about <category name>", the heading linking to the category's view, and SHALL show a "See all" link to the same view on the heading's row. Its videos SHALL be shown as the same grid of cards as the other home sections, in the order the home response lists them.

#### Scenario: Category section shown
- **WHEN** the home response holds a category section for "Chess" with 2 videos
- **THEN** the home view shows "More about Chess" below "Continue watching" and above "Quick watches", listing both videos

#### Scenario: Heading opens the category
- **WHEN** a user activates the "More about Chess" heading
- **THEN** the application navigates to the "Chess" view

#### Scenario: See all opens the category
- **WHEN** a user activates "See all" in the "More about Chess" section
- **THEN** the application navigates to the "Chess" view

#### Scenario: Sections in response order
- **WHEN** the home response holds category sections for "Science" then "Chess"
- **THEN** "More about Science" is shown above "More about Chess"

#### Scenario: No category sections
- **WHEN** the home response holds no category sections
- **THEN** no "More about" section is shown
