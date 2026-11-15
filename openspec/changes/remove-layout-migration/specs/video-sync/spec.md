## MODIFIED Requirements

### Requirement: Sync Repairs Missing Metadata
A video sync SHALL regenerate the metadata of a Downloaded video with a healthy file that has no metadata recorded as populated, without re-downloading it, resolving its `sorttitle` from its playlist position or, for a channel video, its publish date. A video still stored in the previous per-video-folder layout SHALL be left as it is: no metadata is regenerated or written for it and nothing is moved. A failed regeneration SHALL leave the video unchanged and be retried by its next sync.

#### Scenario: Downloaded video has no recorded metadata
- **WHEN** a video sync runs for a Downloaded playlist video with no metadata recorded as populated
- **THEN** its metadata is generated again with its `sorttitle` from its playlist position, without re-downloading its file

#### Scenario: Downloaded video already has recorded metadata
- **WHEN** a video sync runs for a Downloaded video that already has metadata recorded as populated
- **THEN** its metadata is not regenerated

#### Scenario: Video still in the previous layout
- **WHEN** a video sync runs for a Downloaded video with no recorded metadata whose recorded file is still in a per-video folder or a flat file
- **THEN** no metadata is generated or written for it, its files stay where they are, and no new filename is recorded

#### Scenario: Metadata still unavailable
- **WHEN** a video sync runs for a Downloaded video with no recorded metadata and YouTube returns no metadata for it
- **THEN** the video is left unchanged
