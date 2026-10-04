## MODIFIED Requirements

### Requirement: Metadata Generated On Successful Download
The system SHALL fetch a video's metadata from the YouTube Data API and write
its `movie.nfo` file into the video's own folder **before** the video's media
file is downloaded into that folder, so that a media scanner watching the
folder never sees the media file without its `movie.nfo` next to it. Once the
download succeeds, the system SHALL write `movie.nfo` again with the final
thumbnail reference and record the metadata as generated.

#### Scenario: Video downloaded and metadata available
- **WHEN** a video download completes successfully and its YouTube metadata can be fetched
- **THEN** its `movie.nfo` file is generated in its own folder and its metadata is recorded as generated

#### Scenario: movie.nfo precedes the media file
- **WHEN** a video is downloaded and its YouTube metadata can be fetched
- **THEN** its `movie.nfo` file exists in the video's folder before the video's media file first appears in that folder

#### Scenario: Thumbnail reference reflects the downloaded thumbnail
- **WHEN** a video downloads successfully but no thumbnail file was saved alongside it
- **THEN** the final `movie.nfo` omits `thumb`, even if the `movie.nfo` written before the download referenced one

### Requirement: Metadata Generation Failure Does Not Affect Download Outcome
The system SHALL NOT fail, retry, or otherwise alter a video download's
outcome when generating its metadata fails; a video download's success is
determined solely by whether its file was downloaded. When the YouTube
metadata cannot be fetched before the download, the system SHALL still
download the video and SHALL try to generate its metadata once more after the
download succeeds. On a metadata generation failure, the system SHALL write
no `movie.nfo` file and record no completed metadata for that video. A
download that does not succeed SHALL NOT record metadata as generated and
SHALL NOT leave a `movie.nfo` file in the video's folder.

#### Scenario: YouTube metadata fetch fails after a successful download
- **WHEN** a video's YouTube metadata cannot be fetched before or after its download, and its file finishes downloading successfully
- **THEN** the video is still marked as successfully downloaded, no `movie.nfo` file is written, and no metadata is recorded as generated for it

#### Scenario: Metadata fetch fails before the download but succeeds after it
- **WHEN** a video's YouTube metadata cannot be fetched before its download, its file finishes downloading successfully, and the metadata can be fetched afterwards
- **THEN** its `movie.nfo` file is generated in its own folder and its metadata is recorded as generated

#### Scenario: Download fails after movie.nfo was written
- **WHEN** a video's `movie.nfo` was written before its download and the download then fails
- **THEN** no metadata is recorded as generated for that video and no `movie.nfo` file remains in its folder
