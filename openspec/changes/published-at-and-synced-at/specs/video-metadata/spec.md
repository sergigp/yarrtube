## ADDED Requirements

### Requirement: Publish Time Is Recorded
The system SHALL record, with a video's generated metadata, the full publish timestamp YouTube reports for that video. The `premiered` and `year` values in `movie.nfo` SHALL be derived from that recorded publish timestamp.

#### Scenario: Metadata generated
- **WHEN** a video's metadata is generated from YouTube metadata reporting a publish timestamp
- **THEN** that full timestamp is recorded with the video's metadata, and `movie.nfo` has `premiered` as its `YYYY-MM-DD` date and `year` as its year

#### Scenario: Metadata generated before publish time was tracked
- **WHEN** the system upgrades storage that holds already generated metadata
- **THEN** each of those records gets a publish timestamp at midnight UTC on its previously recorded publish date

### Requirement: Metadata Generation Times Are Recorded
The system SHALL record, with a video's generated metadata, when it was first generated and when it was last generated. Generating metadata again for a video that already has it SHALL update the last generated time and SHALL keep the first generated time.

#### Scenario: Metadata generated for the first time
- **WHEN** a video's metadata is generated and none was recorded for it before
- **THEN** its first generated time and last generated time are both the generation time

#### Scenario: Metadata generated again
- **WHEN** a video that already has recorded metadata has its metadata generated again (for example, after a redownload)
- **THEN** its last generated time becomes the new generation time and its first generated time is unchanged
