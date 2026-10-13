## MODIFIED Requirements

### Requirement: Collapsible Video Details
The video detail pane SHALL always show the video's title and its meta line. The rest of the pane (the status indicator when the video is not downloaded, the description, the storage location and the "Open on YouTube" link) SHALL sit in a section the user can expand and collapse. The storage location SHALL be the folder holding the video's file (its source's storage path joined with the file's folder, for example `channels/some-channel/Season 2026`), never the file name itself, so episode codes do not appear in the UI. This section SHALL start collapsed on viewports narrower than the desktop breakpoint and expanded at or above it. The pane SHALL NOT show a quality indicator, and SHALL NOT show a status indicator for a downloaded video.

#### Scenario: Details start collapsed on mobile
- **WHEN** a user on a mobile-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the video's title, its meta line and an expand control, and hides the description, storage location and YouTube link

#### Scenario: Expanding details
- **WHEN** a user activates the expand control in a collapsed detail pane
- **THEN** the description, storage location and "Open on YouTube" link become visible, and the control collapses them again when activated

#### Scenario: Details start expanded on desktop
- **WHEN** a user on a desktop-width viewport opens a playlist or channel detail view
- **THEN** the detail pane shows the title, meta line, description, storage location and YouTube link without needing to be expanded

#### Scenario: Storage location of a downloaded episode
- **WHEN** a downloaded video's recorded filename is `Season 2026/S2026E031501 - Title.mp4` in a channel stored at `channels/some-channel`
- **THEN** the detail pane shows `channels/some-channel/Season 2026` and not the file name

#### Scenario: Storage location of a video not yet downloaded
- **WHEN** a video has no recorded filename
- **THEN** the detail pane shows its source's storage path

#### Scenario: Downloaded video shows no status indicator
- **WHEN** a user expands the details of a downloaded video
- **THEN** no status or quality indicator is shown

#### Scenario: Pending video shows its status
- **WHEN** a user expands the details of a video that is pending, downloading, retrying or errored
- **THEN** a status indicator shows that state, and no quality indicator is shown
