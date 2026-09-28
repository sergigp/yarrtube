## ADDED Requirements

### Requirement: Video Detail Meta Line
The video detail pane SHALL show a meta line directly below the title, visible whether the details section is expanded or collapsed. The line SHALL read `<channel name> · Published <date> · Synced <relative time>`:
- The channel name SHALL appear only in a playlist detail view.
- The publish time SHALL be shown as a date, without a time of day.
- The sync time SHALL be shown relative to now (for example "2d ago"), and its full date and time SHALL be available on hover.
- Any part whose value is absent SHALL be omitted along with its separator. When every part is absent, the line SHALL not be shown.

#### Scenario: Downloaded video in a playlist view
- **WHEN** a user selects a downloaded video with generated metadata in a playlist detail view
- **THEN** the meta line shows the video's channel name, "Published" followed by its publish date, and "Synced" followed by how long ago it was synced

#### Scenario: Downloaded video in a channel view
- **WHEN** a user selects a downloaded video with generated metadata in a channel detail view
- **THEN** the meta line shows "Published" with its publish date and "Synced" with how long ago it was synced, without the channel name

#### Scenario: Meta line on a collapsed mobile pane
- **WHEN** a user on a mobile-width viewport opens a detail view with the details section collapsed
- **THEN** the meta line is visible below the title

#### Scenario: Video without metadata or sync time
- **WHEN** a user selects a pending video that has neither generated metadata nor a sync time
- **THEN** no meta line is shown

### Requirement: Video Detail Description
The expandable section of the video detail pane SHALL show the video's description, preserving its line breaks and rendering web URLs in it as links that open in a new tab. The description SHALL be clamped to about four lines, with a "Show more" control, shown only when the text exceeds the clamp, that reveals the rest and can collapse it again. When the video has no description, or an empty one, no description block SHALL be shown.

#### Scenario: Long description
- **WHEN** a user expands the details of a video whose description is longer than four lines
- **THEN** the first four lines are shown followed by a "Show more" control, and activating it reveals the whole description

#### Scenario: Short description
- **WHEN** a user expands the details of a video whose description fits in four lines
- **THEN** the whole description is shown and no "Show more" control is present

#### Scenario: Link in the description
- **WHEN** a user activates a URL inside a video's description
- **THEN** that URL opens in a new tab

#### Scenario: Video without a description
- **WHEN** a user expands the details of a video with no description
- **THEN** no description block is shown

## MODIFIED Requirements

### Requirement: Video Detail Title Prominence
In the video detail pane, the video's title SHALL occupy its own full-width row. No other elements SHALL share that row at any viewport size, apart from the channel avatar in a channel detail view.

#### Scenario: Long title on a mobile viewport
- **WHEN** a user on a mobile-width viewport selects a video with a long title
- **THEN** the title wraps across the full width of the detail pane, and the meta line appears below it

### Requirement: Collapsible Video Details
The video detail pane SHALL always show the video's title and its meta line. The rest of the pane (the status indicator when the video is not downloaded, the description, the file path and the "Open on YouTube" link) SHALL sit in a section the user can expand and collapse. This section SHALL start collapsed on viewports narrower than the desktop breakpoint and expanded at or above it. The pane SHALL NOT show a quality indicator, and SHALL NOT show a status indicator for a downloaded video.

#### Scenario: Details start collapsed on mobile
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and an expand control, and hides the description, path and YouTube link

#### Scenario: Expanding details
- **WHEN** a user activates the expand control in a collapsed detail pane
- **THEN** the description, file path and "Open on YouTube" link become visible, and the control collapses them again when activated

#### Scenario: Details start expanded on desktop
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the title, meta line, description, path and YouTube link without needing to be expanded

#### Scenario: Downloaded video shows no status indicator
- **WHEN** a user expands the details of a downloaded video
- **THEN** no status or quality indicator is shown

#### Scenario: Pending video shows its status
- **WHEN** a user expands the details of a video that is pending, downloading, retrying or errored
- **THEN** a status indicator shows that state, and no quality indicator is shown
