# player-chapters Specification

## Purpose

Presents a playing video's chapters in the web player as a dedicated,
readable navigation surface, so the user can see where they are and jump
to any chapter.

## Requirements

### Requirement: Chapter Rail Under The Player
When the video in a playlist, channel or category player is downloaded and has chapters, the player SHALL show a chapter rail directly below the video: one card per chapter, in playback order, each showing the chapter's start time and title. Cards SHALL have the same width; titles that don't fit SHALL be cut with an ellipsis and offered in full on hover. When there are more cards than fit, the rail SHALL scroll sideways.

#### Scenario: Video with chapters
- **WHEN** a user selects a downloaded video with chapters "Intro" (0–75 s), "Game 1" (75–1037 s) and "Game 2" (1037–2201 s)
- **THEN** below the video the rail shows three cards, in order: "0:00 Intro", "1:15 Game 1" and "17:17 Game 2"

#### Scenario: Hour-long video
- **WHEN** the selected video has a chapter starting at 3725 s
- **THEN** its card shows the start time "1:02:05"

#### Scenario: Category player
- **WHEN** a user plays a downloaded video with chapters from a category
- **THEN** the rail is shown below the video the same way as in a playlist or channel

### Requirement: No Rail Without Chapters
The player SHALL show no chapter rail, and nothing in its place, when the selected video has no chapters, its chapters were never checked, or it is not downloaded.

#### Scenario: Video without chapters
- **WHEN** a user selects a downloaded video with an empty chapter list
- **THEN** no chapter rail and no chapter-related message are shown

#### Scenario: Video not downloaded
- **WHEN** a user selects a video that has chapters but is not downloaded
- **THEN** no chapter rail is shown

### Requirement: Current Chapter Highlighted
The rail SHALL mark the chapter that contains the video's current playback position as the current one, visually distinct and exposed to assistive technology, with a fill showing how far playback is through it. As playback moves into another chapter, the mark SHALL move with it and the rail SHALL scroll that chapter's card into view.

#### Scenario: Playback inside a chapter
- **WHEN** a video with chapters "Game 1" (0–1037 s) and "Game 2" (1037–2201 s) is at 1500 s
- **THEN** "Game 2" is marked as the current chapter and its fill shows about 40% progress

#### Scenario: Playback reaches the next chapter
- **WHEN** playback crosses from "Game 1" into "Game 2" and the "Game 2" card is scrolled out of view
- **THEN** "Game 2" becomes the current chapter and its card scrolls into view

#### Scenario: Playback outside every chapter
- **WHEN** the playback position falls in no chapter (e.g. before the first chapter starts)
- **THEN** no chapter is marked as current

### Requirement: Clicking A Chapter Seeks To It
Activating a chapter card SHALL move the video's playback position to that chapter's start and play the video if it was paused.

#### Scenario: Jumping to a chapter while paused
- **WHEN** the video is paused at 30 s and the user clicks the "Game 2" card starting at 1037 s
- **THEN** the video is playing from 1037 s and "Game 2" is the current chapter

### Requirement: Player Makes Room For The Rail On Desktop
At or above the desktop breakpoint, when the chapter rail is shown, the video and the rail together SHALL stay within the same maximum height the video alone is limited to without the rail. A video already shorter than that limit SHALL NOT be shrunk; the rail is added below it.

#### Scenario: Video limited by the maximum height
- **WHEN** a user on a short desktop-width viewport, where a video without chapters is shown at the maximum player height, selects a downloaded video with chapters
- **THEN** the video and the rail below it together take no more than that maximum height

#### Scenario: Video below the maximum height
- **WHEN** a user on a desktop-width viewport, where the video is limited by its width and below the maximum player height, selects a downloaded video with chapters
- **THEN** the video keeps its size and the rail is shown below it

### Requirement: Rail Pinned With The Mobile Player
Below the desktop breakpoint, the chapter rail SHALL stay pinned directly under the pinned player while the page scrolls, as a single row of cards swiped sideways.

#### Scenario: Scrolling on a mobile viewport
- **WHEN** a user on a mobile-width viewport scrolls down a detail view whose video has chapters
- **THEN** the player and the chapter rail stay pinned together below the application header

### Requirement: Full Chapter List On Mobile
Below the desktop breakpoint, the rail SHALL end with a control named "All chapters" that opens a sheet listing every chapter, one per row, with its start time and full title, the current chapter marked. Picking a chapter there SHALL seek to its start, play the video and close the sheet. At or above the desktop breakpoint, this control SHALL NOT be shown.

#### Scenario: Picking a chapter from the list
- **WHEN** a user on a mobile-width viewport taps "All chapters" and then "Game 2"
- **THEN** the sheet closes and the video is playing from the start of "Game 2"

#### Scenario: Current chapter in the list
- **WHEN** the user opens the list while "Game 1" is playing
- **THEN** "Game 1" is marked as the current chapter in the list

#### Scenario: Desktop viewport
- **WHEN** a user on a desktop-width viewport selects a video with chapters
- **THEN** the rail shows no "All chapters" control
