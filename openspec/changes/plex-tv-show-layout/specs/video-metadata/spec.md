## MODIFIED Requirements

### Requirement: Metadata File Named movie.nfo
The system SHALL write a downloaded video's metadata sidecar as an episode NFO named after the video's file, with the `.nfo` extension in place of the video extension, in the video's season folder (for example `Season 2026/S2026E031501 - Title.nfo`).

#### Scenario: Metadata generated
- **WHEN** a video's metadata is successfully generated
- **THEN** it is written to a file sharing the video file's episode base name with the `.nfo` extension, in the same season folder

### Requirement: Metadata Generated On Successful Download
The system SHALL fetch a video's metadata from the YouTube Data API and write
its episode NFO into the video's season folder **before** the video's media
file is downloaded into that folder, so that a media scanner watching the
folder never sees the media file without its NFO next to it. Once the
download succeeds, the system SHALL write the NFO again with the final
thumbnail reference and record the metadata as generated.

#### Scenario: Video downloaded and metadata available
- **WHEN** a video download completes successfully and its YouTube metadata can be fetched
- **THEN** its episode NFO is generated next to its file and its metadata is recorded as generated

#### Scenario: movie.nfo precedes the media file
- **WHEN** a video is downloaded and its YouTube metadata can be fetched
- **THEN** its episode NFO exists in the season folder before the video's media file first appears there

#### Scenario: Thumbnail reference reflects the downloaded thumbnail
- **WHEN** a video downloads successfully but no thumbnail file was saved alongside it
- **THEN** the final NFO omits `thumb`, even if the NFO written before the download referenced one

### Requirement: Metadata Generation Failure Does Not Affect Download Outcome
The system SHALL NOT fail, retry, or otherwise alter a video download's
outcome when generating its metadata fails; a video download's success is
determined solely by whether its file was downloaded. When the YouTube
metadata cannot be fetched before the download, the system SHALL still
download the video under its episode name (numbering does not depend on the
Data API, see `tv-show-layout`) and SHALL try to generate its metadata once
more after the download succeeds. On a metadata generation failure, the
system SHALL write no NFO file and record no completed metadata for that
video. A download that does not succeed SHALL NOT record metadata as
generated and SHALL NOT leave an NFO file in the season folder.

#### Scenario: YouTube metadata fetch fails after a successful download
- **WHEN** a video's YouTube metadata cannot be fetched before or after its download, and its file finishes downloading successfully
- **THEN** the video is still marked as successfully downloaded under its episode name, no NFO file is written, and no metadata is recorded as generated for it

#### Scenario: Metadata fetch fails before the download but succeeds after it
- **WHEN** a video's YouTube metadata cannot be fetched before its download, its file finishes downloading successfully, and the metadata can be fetched afterwards
- **THEN** its episode NFO is generated next to its file and its metadata is recorded as generated

#### Scenario: Download fails after movie.nfo was written
- **WHEN** a video's episode NFO was written before its download and the download then fails
- **THEN** no metadata is recorded as generated for that video and no NFO file remains in the season folder

### Requirement: NFO Field Mapping
The system SHALL populate a generated episode NFO (root element
`episodedetails`) from the video's fetched YouTube metadata as follows:
`title` from the video's title, `plot` from its description, `season` and
`episode` from the video's recorded episode number, `aired` and `premiered`
from the full publish date (`YYYY-MM-DD`), `year` from the publish year,
`studio` and `director` from its channel name. The system SHALL also include
a `uniqueid` of type `youtube` set to the video's YouTube ID, and, when a
thumbnail file was saved alongside the video, a `thumb` referencing it.

#### Scenario: Metadata generated with a thumbnail present
- **WHEN** a video's metadata is generated and a thumbnail file exists for it
- **THEN** the resulting NFO contains `title`, `plot`, `season`, `episode`, `aired`, `premiered`, `year`, `studio`, `director`, a `uniqueid` of type `youtube` holding the video's YouTube ID, and a `thumb` referencing the saved thumbnail filename

#### Scenario: Metadata generated without a thumbnail
- **WHEN** a video's metadata is generated and no thumbnail file exists for it
- **THEN** the resulting NFO omits `thumb` but still contains every other mapped field

### Requirement: XML Escaping
The system SHALL escape every text value written into an episode NFO (at
minimum `&`, `<`, `>`, `"`, `'`) so the file is always well-formed XML,
regardless of characters present in the video's title, description,
channel name, or tags.

#### Scenario: Title contains XML-significant characters
- **WHEN** a video's title, description, channel name, or a tag contains a character that is illegal unescaped in XML
- **THEN** the generated NFO contains that value correctly escaped and remains well-formed XML
