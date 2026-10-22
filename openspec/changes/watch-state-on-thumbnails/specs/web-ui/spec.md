## MODIFIED Requirements

### Requirement: Home Page Layout
The home view SHALL display up to three sections, in this order: "Continue watching", "Quick watches" and "Latest videos", loaded together in one request. A video SHALL appear in at most one section. "Latest videos" SHALL display up to 18 recently synced videos. "Continue watching" SHALL display up to 6 videos the user has started and not finished recently. "Quick watches" SHALL display up to 6 short unwatched videos. "Continue watching" and "Quick watches" SHALL be hidden when they have no videos, including while their videos are loading or when they cannot be loaded. Each section SHALL display its videos as a grid of cards, each showing the video's thumbnail with its title below it. For videos from a tracked channel, the card SHALL also show the channel's avatar and name.

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

#### Scenario: A video appears in one section only
- **WHEN** a video qualifies for more than one home section
- **THEN** it is shown only in the first of them, in section order

#### Scenario: Empty sections are hidden
- **WHEN** no video qualifies for "Continue watching" or "Quick watches"
- **THEN** that section, including its heading, is not shown

#### Scenario: Optional sections that fail to load are hidden
- **WHEN** the videos for "Continue watching" or "Quick watches" are still loading or cannot be loaded
- **THEN** that section, including its heading, is not shown, and "Latest videos" still renders (showing the load error when there is one)

### Requirement: Watched Tick On Video Thumbnails
Every video thumbnail in the playlist detail, channel detail, category and home views SHALL display a tick when that video is watched, and SHALL NOT display one when it is unwatched.

#### Scenario: Watched video thumbnail
- **WHEN** a watched video is listed in a playlist, channel, category or home view
- **THEN** its thumbnail displays a tick

#### Scenario: Unwatched video thumbnail
- **WHEN** an unwatched video is listed
- **THEN** its thumbnail does not display a tick

## ADDED Requirements

### Requirement: Watch Progress On Video Thumbnails
Every video thumbnail in the playlist detail, channel detail, category and home views SHALL show the video as in progress when it is unwatched, its saved playback position is greater than 0 and its duration is known. An in-progress thumbnail SHALL show a progress bar along its bottom edge, proportional to the saved position over the duration, and SHALL show the time left instead of the video's duration. The time left is the duration minus the saved position: below one minute it SHALL read `<1m left`; below one hour, `<minutes>m left` with the minutes rounded down; from one hour, `<hours>h <minutes>m left`, or `<hours>h left` when the minutes are 0. The duration badge and the watched tick SHALL NOT overlap the progress bar. A thumbnail that is not in progress SHALL show no progress bar and SHALL show the video's duration as before.

#### Scenario: In-progress video in a channel or playlist list
- **WHEN** an unwatched video with a saved position of 120 seconds and a duration of 600 seconds is listed in a channel or playlist view
- **THEN** its thumbnail shows a progress bar at 20% and the badge reads `8m left`

#### Scenario: In-progress video on a home or category card
- **WHEN** an unwatched video with a saved position greater than 0 and a known duration is shown in any home section or in a category view
- **THEN** its card's thumbnail shows a progress bar and the time left

#### Scenario: Time left from one hour
- **WHEN** an in-progress video has 3,900 seconds left
- **THEN** its badge reads `1h 5m left`

#### Scenario: Time left in whole hours
- **WHEN** an in-progress video has 3,600 seconds left
- **THEN** its badge reads `1h left`

#### Scenario: Under a minute left
- **WHEN** an in-progress video has 45 seconds left
- **THEN** its badge reads `<1m left`

#### Scenario: Unstarted video
- **WHEN** an unwatched video with a saved position of 0 is listed
- **THEN** its thumbnail shows no progress bar and its badge shows the video's duration

#### Scenario: Watched video
- **WHEN** a watched video is listed
- **THEN** its thumbnail shows no progress bar and its badge shows the video's duration

#### Scenario: Unknown duration
- **WHEN** an unwatched video with a saved position greater than 0 has no known duration
- **THEN** its thumbnail shows no progress bar and no duration badge

### Requirement: Watched Videos Are De-emphasized
In the playlist detail, channel detail, category and home views, a watched video SHALL be shown de-emphasized: its thumbnail dimmed and its title in a muted color. The watched tick SHALL NOT be dimmed. On home and category cards, hovering a watched card SHALL show its thumbnail at full strength. In a channel or playlist video list, the selected video's title SHALL keep its selected highlight even when the video is watched. Unwatched videos, in progress or not, SHALL NOT be de-emphasized.

#### Scenario: Watched video in a list
- **WHEN** a watched video is listed in a channel or playlist view
- **THEN** its thumbnail is dimmed and its title is muted, while its tick is shown at full strength

#### Scenario: Watched card hovered
- **WHEN** the user hovers a watched video's card in a home section or a category view
- **THEN** its thumbnail is shown at full strength

#### Scenario: Selected watched video
- **WHEN** the selected video in a channel or playlist view is watched
- **THEN** its title keeps the selected highlight

#### Scenario: Unwatched video
- **WHEN** an unwatched video is listed
- **THEN** neither its thumbnail nor its title is de-emphasized
