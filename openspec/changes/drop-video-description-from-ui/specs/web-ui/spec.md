## ADDED Requirements

### Requirement: Video Detail Status And Location
The video detail pane SHALL always show, below the meta line, the video's storage location and, when the video is not downloaded, a status indicator, at every viewport size and with no control to expand or collapse them. The storage location SHALL be the folder holding the video's file, never the file name itself, so episode codes do not appear in the UI. The pane SHALL NOT show a quality indicator, a status indicator for a downloaded video, or the video's YouTube description.

#### Scenario: Details on a mobile viewport
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and its storage location, with no "More" or "Less" control

#### Scenario: Details on a desktop viewport
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and its storage location, with no "More" or "Less" control

#### Scenario: Storage location of a downloaded episode
- **WHEN** a downloaded video's recorded filename is `Season 2026/S2026E031501 - Title.mp4` in a channel stored at `channels/some-channel`
- **THEN** the detail pane shows `channels/some-channel/Season 2026` and not the file name

#### Scenario: Storage location of a video not yet downloaded
- **WHEN** a video has no recorded filename
- **THEN** the detail pane shows its source's storage path

#### Scenario: Downloaded video shows no status indicator
- **WHEN** a user selects a downloaded video
- **THEN** no status or quality indicator is shown

#### Scenario: Pending video shows its status
- **WHEN** a user selects a video that is pending, downloading, retrying or errored
- **THEN** a status indicator shows that state, and no quality indicator is shown

## MODIFIED Requirements

### Requirement: Video Detail Links To YouTube
A selected video's detail pane SHALL show, at the end of its title row, an icon-only link whose accessible name and tooltip are "Open video on YouTube", which opens that video on YouTube in a new tab.

#### Scenario: Opening a video on YouTube
- **WHEN** a user activates the "Open video on YouTube" link for a selected video
- **THEN** a new tab opens to that video's page on youtube.com

#### Scenario: Visible while details are collapsed
- **WHEN** a user on a mobile-width viewport selects a video
- **THEN** the "Open video on YouTube" link is visible in the title row

### Requirement: Video Detail Meta Line
The video detail pane SHALL show a meta line directly below the title. The line SHALL read `Watched · <channel name> · Published <date> · Synced <relative time>`:
- "Watched" SHALL appear only when the video is watched.
- The channel name SHALL appear only in a playlist detail view.
- The publish time SHALL be shown as a date, without a time of day.
- The sync time SHALL be shown relative to now (for example "2d ago"), and its full date and time SHALL be available on hover. It SHALL be shown only at or above the desktop breakpoint.
- Any part whose value is absent SHALL be omitted along with its separator. When every part is absent, the line SHALL not be shown.

#### Scenario: Downloaded video in a playlist view
- **WHEN** a user on a desktop-width viewport selects a downloaded, unwatched video with generated metadata in a playlist detail view
- **THEN** the meta line shows the video's channel name, "Published" followed by its publish date, and "Synced" followed by how long ago it was synced

#### Scenario: Downloaded video in a channel view
- **WHEN** a user on a desktop-width viewport selects a downloaded, unwatched video with generated metadata in a channel detail view
- **THEN** the meta line shows "Published" with its publish date and "Synced" with how long ago it was synced, without the channel name

#### Scenario: Watched video
- **WHEN** a user selects a watched video
- **THEN** the meta line starts with "Watched", followed by the other parts

#### Scenario: Sync time hidden on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a synced video
- **THEN** the meta line shows no "Synced" part and no separator for it

#### Scenario: Meta line on a collapsed mobile pane
- **WHEN** a user on a mobile-width viewport opens a detail view
- **THEN** the meta line is visible below the title

#### Scenario: Video without metadata or sync time
- **WHEN** a user selects a pending video that has neither generated metadata nor a sync time
- **THEN** no meta line is shown

## REMOVED Requirements

### Requirement: Collapsible Video Details
**Reason**: With the description gone, the section would hold only the storage location and the status indicator, too little to collapse.
**Migration**: Replaced by "Video Detail Status And Location".

### Requirement: Video Detail Description
**Reason**: YouTube descriptions are mostly spam, and the "Open video on YouTube" link reaches the full text.
**Migration**: Use the "Open video on YouTube" link. Media servers still get the description as the NFO `plot`.
