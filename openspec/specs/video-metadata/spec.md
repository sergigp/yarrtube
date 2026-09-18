# video-metadata Specification

## Purpose

Generates the Plex-facing `movie.nfo` metadata sidecar for each downloaded
video from YouTube's own video data, and tracks whether that metadata has
been successfully generated so a failed or skipped generation can be
retried later without re-downloading the video.

## Requirements

### Requirement: Metadata File Named movie.nfo
The system SHALL write a downloaded video's metadata sidecar as `movie.nfo`
in that video's own output folder.

#### Scenario: Metadata generated
- **WHEN** a video's metadata is successfully generated
- **THEN** it is written to a file named `movie.nfo` in that video's own folder

### Requirement: Metadata Generated On Successful Download
The system SHALL fetch a video's metadata from the YouTube Data API and
generate its `movie.nfo` file immediately after that video's download
succeeds.

#### Scenario: Video downloaded and metadata available
- **WHEN** a video download completes successfully and its YouTube metadata can be fetched
- **THEN** its `movie.nfo` file is generated in its own folder

### Requirement: Metadata Generation Failure Does Not Affect Download Outcome
The system SHALL NOT fail, retry, or otherwise alter a video download's
outcome when generating its metadata fails; a video download's success is
determined solely by whether its file was downloaded. On a metadata
generation failure, the system SHALL write no `movie.nfo` file and record
no completed metadata for that video.

#### Scenario: YouTube metadata fetch fails after a successful download
- **WHEN** a video's file finishes downloading successfully but its YouTube metadata cannot be fetched
- **THEN** the video is still marked as successfully downloaded, no `movie.nfo` file is written, and no metadata is recorded as generated for it

### Requirement: NFO Field Mapping
The system SHALL populate a generated `movie.nfo` from the video's fetched
YouTube metadata as follows: `title` from the video's title, `plot` from
its description, `studio` and `director` from its channel name, `premiered`
from the full publish date (`YYYY-MM-DD`), and `year` from the publish
year. The system SHALL also include a `uniqueid` set to the video's YouTube
ID, and, when a thumbnail file was saved alongside the video, a `thumb`
referencing it.

#### Scenario: Metadata generated with a thumbnail present
- **WHEN** a video's metadata is generated and a thumbnail file exists for it
- **THEN** the resulting `movie.nfo` contains `title`, `plot`, `studio`, `director`, `premiered`, `year`, a `uniqueid` of type `youtube` holding the video's YouTube ID, and a `thumb` referencing the saved thumbnail filename

#### Scenario: Metadata generated without a thumbnail
- **WHEN** a video's metadata is generated and no thumbnail file exists for it
- **THEN** the resulting `movie.nfo` omits `thumb` but still contains every other mapped field

### Requirement: XML Escaping
The system SHALL escape every text value written into `movie.nfo` (at
minimum `&`, `<`, `>`, `"`, `'`) so the file is always well-formed XML,
regardless of characters present in the video's title, description,
channel name, or tags.

#### Scenario: Title contains XML-significant characters
- **WHEN** a video's title, description, channel name, or a tag contains a character that is illegal unescaped in XML
- **THEN** the generated `movie.nfo` contains that value correctly escaped and remains well-formed XML

### Requirement: Genre From Category Mapping
The system SHALL map a video's YouTube category ID to a genre name using a
fixed, built-in mapping, and include it as `genre` in the generated
`movie.nfo`. When the category ID has no mapped name, the system SHALL
omit `genre` rather than failing metadata generation.

#### Scenario: Video has a mapped category
- **WHEN** a video's YouTube category ID has a corresponding genre name in the built-in mapping
- **THEN** the generated `movie.nfo` includes that genre name as `genre`

#### Scenario: Video has an unmapped category
- **WHEN** a video's YouTube category ID has no corresponding entry in the built-in mapping
- **THEN** the generated `movie.nfo` omits `genre` and metadata generation still succeeds

### Requirement: Tags From YouTube
The system SHALL include one `tag` element per entry in the video's
YouTube tags, when any are present, and SHALL include no `tag` element
when the video has none.

#### Scenario: Video has tags
- **WHEN** a video's YouTube metadata includes one or more tags
- **THEN** the generated `movie.nfo` contains one `tag` element per tag

#### Scenario: Video has no tags
- **WHEN** a video's YouTube metadata includes no tags
- **THEN** the generated `movie.nfo` contains no `tag` elements

### Requirement: Plot Truncation
The system SHALL truncate a video's description to at most 500 characters
when generating `plot`, cutting at the last whole word within that limit
and appending an ellipsis, and SHALL leave a description of 500 characters
or fewer unchanged.

#### Scenario: Description exceeds the limit
- **WHEN** a video's description is longer than 500 characters
- **THEN** the generated `plot` is truncated to at most 500 characters, ending at a word boundary with a trailing ellipsis

#### Scenario: Description is within the limit
- **WHEN** a video's description is 500 characters or fewer
- **THEN** the generated `plot` contains it unchanged

### Requirement: Sorttitle Reflects Playlist Order Or Publish Date
The system SHALL derive `sorttitle` as a fixed-width, zero-padded numeric
prefix followed by the video's title: for a video belonging to a
YouTube-linked playlist with a recorded playlist position, that position;
for every other video (a custom-playlist video, or a channel-tracked
video), a prefix derived from its YouTube publish date instead.

#### Scenario: Video belongs to a YouTube-linked playlist with a known position
- **WHEN** a video's metadata is generated and it has a recorded position within its owning YouTube-linked playlist
- **THEN** its `sorttitle` is prefixed with that position, zero-padded to a fixed width

#### Scenario: Video belongs to a custom playlist
- **WHEN** a video's metadata is generated and it belongs to a custom playlist with no recorded playlist position
- **THEN** its `sorttitle` is prefixed with a zero-padded value derived from its YouTube publish date

#### Scenario: Video belongs to a tracked channel
- **WHEN** a video's metadata is generated and it belongs to a tracked channel
- **THEN** its `sorttitle` is prefixed with a zero-padded value derived from its YouTube publish date, not the channel's recency rank

### Requirement: Metadata Completeness Is Recorded
The system SHALL record that a video's metadata has been generated only
after its `movie.nfo` file has been successfully written, and SHALL treat
that record as the sole source of truth for whether a video's metadata is
populated.

#### Scenario: Metadata generation succeeds
- **WHEN** a video's `movie.nfo` file is successfully written
- **THEN** the system records that video's metadata as populated

#### Scenario: Checking whether a video's metadata is populated
- **WHEN** the system needs to know whether a video's metadata has been generated
- **THEN** it consults the recorded completeness state rather than inspecting the video's folder or `movie.nfo` file
