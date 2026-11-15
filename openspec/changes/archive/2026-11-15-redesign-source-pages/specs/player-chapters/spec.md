## ADDED Requirements

### Requirement: Chapter List Beside The Summary
At or above the desktop breakpoint, when the selected video in a playlist, channel or category view is downloaded and has chapters, its info SHALL list them in a compact panel labeled "Chapters", beside the AI summary panel (or on its own without a summary): one row per chapter, in playback order, with its start time and its title, cut with an ellipsis when it doesn't fit and offered in full on hover. The list SHALL scroll within its panel when it holds more rows than fit.

#### Scenario: Video with chapters on desktop
- **WHEN** a user on a desktop-width viewport selects a downloaded video with chapters "Intro" (0–87 s) and "Open centre" (87–300 s)
- **THEN** a "Chapters" panel beside the AI summary lists "0:00 Intro" and "1:27 Open centre", with "Intro" marked current

### Requirement: Chapter List Navigation
The chapter list SHALL mark the row of the chapter containing the playback position as current, visually and to assistive technology. Activating a row SHALL move the playback position to that chapter's start and play the video.

#### Scenario: Picking a chapter from the list
- **WHEN** the user activates "Open centre"
- **THEN** the video is playing from 87 s

## MODIFIED Requirements

### Requirement: Chapter Rail Under The Player
Below the desktop breakpoint, when the video in a playlist, channel or category player is downloaded and has chapters, the player SHALL show a chapter rail directly below the video: one card per chapter, in playback order, each showing the chapter's start time and title. Cards SHALL have the same width; titles that don't fit SHALL be cut with an ellipsis and offered in full on hover. When there are more cards than fit, the rail SHALL scroll sideways. At or above the desktop breakpoint there SHALL be no rail; the chapters are listed as described in "Chapter List Beside The Summary".

#### Scenario: Video with chapters
- **WHEN** a user on a mobile-width viewport selects a downloaded video with chapters "Intro" (0–75 s), "Game 1" (75–1037 s) and "Game 2" (1037–2201 s)
- **THEN** below the video the rail shows three cards, in order: "0:00 Intro", "1:15 Game 1" and "17:17 Game 2"

#### Scenario: Hour-long video
- **WHEN** the selected video has a chapter starting at 3725 s
- **THEN** its card shows the start time "1:02:05"

#### Scenario: Category player
- **WHEN** a user on a mobile-width viewport plays a downloaded video with chapters from a category
- **THEN** the rail is shown below the video the same way as in a playlist or channel

#### Scenario: Desktop viewport
- **WHEN** a user on a desktop-width viewport selects a downloaded video with chapters
- **THEN** no rail is shown under the video

## REMOVED Requirements

### Requirement: Player Makes Room For The Rail On Desktop
**Reason**: There is no rail on desktop any more; the chapters are listed beside the AI summary, below the player.
**Migration**: None; the desktop player keeps its maximum height whether or not the video has chapters.
