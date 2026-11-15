## MODIFIED Requirements

### Requirement: Legacy Layouts Are Not Served As Shows
The system SHALL NOT write any new file in the previous per-video-folder layout. A video whose recorded filename still points into a per-video folder or to a flat file (a library downloaded before the TV-show layout) SHALL remain playable in the web UI and SHALL NOT be moved, reset for redownload or swept by reconcile.

#### Scenario: Unmigrated video
- **WHEN** a reconcile pass finds a downloaded video whose recorded filename is not under a `Season <season>` folder
- **THEN** the pass leaves its files where they are and does not reset it for redownload

#### Scenario: Legacy folder titled like a season
- **WHEN** a legacy video's per-video folder is named after a title starting with `Season ` (e.g. `Season 2 Finale`)
- **THEN** it is treated as legacy: reconcile never sweeps its files
