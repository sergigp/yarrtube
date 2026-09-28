## MODIFIED Requirements

### Requirement: Home Page Layout
The home view SHALL display up to three sections, in this order: "Continue watching", "Quick watches" and "Latest videos". "Latest videos" SHALL display up to 18 recently synced videos. "Continue watching" SHALL display up to 6 videos the user has started and not finished recently, and SHALL show on each card how far the video has been watched. "Quick watches" SHALL display up to 6 short unwatched videos. "Continue watching" and "Quick watches" SHALL be hidden when they have no videos, including while their videos are loading or when they cannot be loaded. Each section SHALL display its videos as a grid of cards, each showing the video's thumbnail with its title below it. For videos from a tracked channel, the card SHALL also show the channel's avatar and name.

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
- **WHEN** at least one short unwatched video has been downloaded
- **THEN** the home view shows a "Quick watches" section below "Continue watching" and above "Latest videos", listing that video

#### Scenario: Sections show a bounded number of videos
- **WHEN** more videos qualify for a section than it shows
- **THEN** "Continue watching" and "Quick watches" each show at most 6 videos, and "Latest videos" shows at most 18

#### Scenario: Empty sections are hidden
- **WHEN** no video qualifies for "Continue watching" or "Quick watches"
- **THEN** that section, including its heading, is not shown

#### Scenario: Optional sections that fail to load are hidden
- **WHEN** the videos for "Continue watching" or "Quick watches" are still loading or cannot be loaded
- **THEN** that section, including its heading, is not shown, and the other sections still render
