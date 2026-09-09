## Purpose

Derives a sanitized, filesystem-safe, human-readable filename for a downloaded video from its title, so saved files are usable outside of raw YouTube metadata and can later support additional naming strategies.

## ADDED Requirements

### Requirement: Whitespace Normalization
The system SHALL collapse consecutive whitespace in a video's title into a single space and SHALL trim leading and trailing whitespace when deriving its output filename.

#### Scenario: Title contains repeated or surrounding whitespace
- **WHEN** a video's title contains multiple consecutive spaces, or leading/trailing whitespace
- **THEN** the derived filename contains single spaces between words and no leading or trailing whitespace

### Requirement: Filesystem-Unsafe Character Replacement
The system SHALL replace characters that are unsafe or reserved on common filesystems (at least `/ \ : * ? " < > |`) with a safe separator character when deriving a filename, instead of leaving them unescaped or relying on `yt-dlp`'s own substitution.

#### Scenario: Title contains a filesystem-reserved character
- **WHEN** a video's title contains one of the reserved characters
- **THEN** the derived filename contains a safe separator in its place and no reserved character

### Requirement: Decorative Symbol Stripping
The system SHALL remove decorative and emoji symbols from a title when deriving a filename, while preserving letters (including accented and other non-ASCII letters), digits, and standard punctuation.

#### Scenario: Title contains decorative or emoji symbols
- **WHEN** a video's title contains emoji or decorative symbol characters
- **THEN** the derived filename omits those characters

#### Scenario: Title contains accented letters
- **WHEN** a video's title contains accented or other non-ASCII letters that are part of ordinary words
- **THEN** the derived filename preserves those letters unchanged

### Requirement: Filename Length Limit
The system SHALL truncate a derived filename so that, together with its file extension, it stays within common filesystem filename length limits, without splitting a multi-byte character.

#### Scenario: Sanitized title exceeds the safe length
- **WHEN** a video's title, once sanitized, would exceed the safe filename length limit
- **THEN** the derived filename is truncated to fit within the limit while remaining valid text

### Requirement: Video ID Omitted By Default
The system SHALL NOT include the video's YouTube ID in the filename derived from its title.

#### Scenario: No naming collision
- **WHEN** a video is downloaded and its sanitized title does not collide with a file already present in the target output directory
- **THEN** the saved file's name is based solely on the sanitized title, without the video ID

### Requirement: Collision Fallback Appends Video ID
The system SHALL detect when a derived filename, ignoring file extension, already matches a file present in the target output directory, and in that case SHALL append the video's YouTube ID to the filename to disambiguate it.

#### Scenario: Two videos sanitize to the same filename in one playlist
- **WHEN** a video's sanitized filename matches the name of a file already present in the same playlist's output directory
- **THEN** the system appends the video's YouTube ID to the filename before saving, so the two files do not collide
