## ADDED Requirements

### Requirement: Preview a YouTube Channel
The system SHALL provide an HTTP endpoint that, given a `channel` value that is either a YouTube channel handle or a YouTube channel URL carrying a handle, looks the channel up on YouTube and returns its handle, its YouTube title, and the URL of its avatar image as YouTube reports it (or no avatar when YouTube reports none), without persisting anything, storing any avatar, publishing any event, or scheduling any task. It applies the same handle extraction and validation as Create Channel.

The preview is independent of what is already tracked: previewing a channel that is already stored SHALL return its YouTube data like any other.

#### Scenario: Previewing a channel by handle
- **WHEN** a request supplies the handle `@somechannel` of a YouTube channel titled "Some Channel" with an avatar
- **THEN** the system returns the handle `@somechannel`, the title "Some Channel" and the avatar's URL, and storage, the avatar store, events and tasks are unchanged

#### Scenario: Previewing a channel by URL
- **WHEN** a request supplies a YouTube channel URL carrying a handle (e.g. `https://www.youtube.com/@somechannel/videos`)
- **THEN** the system returns the extracted handle `@somechannel` together with its YouTube title and avatar URL

#### Scenario: Previewing a channel without an avatar
- **WHEN** a request identifies a YouTube channel for which YouTube reports no avatar
- **THEN** the system returns its handle and title with no avatar

#### Scenario: Previewing an invalid value
- **WHEN** a request omits the value, or supplies an empty value, a handle missing its leading `@`, a URL that is not a recognized YouTube URL, or a YouTube URL without a handle
- **THEN** the system returns a bad request with a meaningful error description without contacting YouTube

#### Scenario: Previewing a nonexistent channel
- **WHEN** a request supplies a handle that does not correspond to an existing, accessible YouTube channel
- **THEN** the system returns a not found response with a meaningful error description

#### Scenario: YouTube unavailable
- **WHEN** the YouTube lookup itself fails
- **THEN** the system returns a bad gateway response with a meaningful error description
