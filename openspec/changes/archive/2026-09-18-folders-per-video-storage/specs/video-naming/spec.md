## MODIFIED Requirements

### Requirement: Video ID Omitted By Default
The system SHALL NOT include the video's YouTube ID in the folder or filenames derived from its title.

#### Scenario: No naming collision
- **WHEN** a video is downloaded and its sanitized title does not collide with an entry already present in the container's output directory
- **THEN** the video's own output folder, and the file saved inside it, are named based solely on the sanitized title, without the video ID

### Requirement: Collision Fallback Appends Video ID
The system SHALL detect when a derived folder name already matches an entry (file or folder) present in the container's output directory, and in that case SHALL append the video's YouTube ID to that folder name to disambiguate it.

#### Scenario: Two videos sanitize to the same filename in one playlist
- **WHEN** a video's sanitized title matches the name of an entry already present in the same playlist's or channel's output directory
- **THEN** the system appends the video's YouTube ID to its own output folder's name before saving into it, so the two videos' folders do not collide
